#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]
#![warn(missing_docs)]
//! parses the .torrent file
//!

use bendy::decoding::{Decoder, Object};
use bendy::serde::from_bytes;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fs::read;
use std::sync::OnceLock;

static TORRENT: OnceLock<Torrent> = OnceLock::new();

/// part of the torrent struct so you know how to parse the data
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum TorrentMode {
    SingleFile, // if single file use Info field `info.length` for file length and `info.name` for file name
    MultipleFile, // if multiple file use Info field `info.files[index].length` and `info.files[index].path` to get name and path of each file and `name` for the directory name
}

impl Default for TorrentMode {
    fn default() -> Self {
        TorrentMode::SingleFile
    }
}

/// main torrent struct, is initilalized during parse_torrent_file
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
struct Torrent {
    announce: String, // url of the tracker (http or udp)
    info: Info,

    //non-encoded info, computed by me
    #[serde(default)]
    //info_hash: Vec<u8>, // 20 byte SHA1 hashvalue of the swarm
    info_hash: [u8; 20],
    #[serde(default)]
    torrent_mode: TorrentMode, // single file or multiple file mode. tells you how to deal with the fields of Info (info.files or info.)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
struct Info {
    name: String, // name of the file, name of the suggested directory if multiple file mode
    #[serde(default)]
    length: u32, // number of bytes of the file
    #[serde(rename = "piece length")]
    piece_length: u32, // number of bytes per piece
    #[serde(with = "serde_bytes")]
    pieces: Vec<u8>, // 20 byte SHA1 hash value of each piece, the files are concatenated in the order they appear in the files list, will need to split based on file length
    #[serde(default)]
    files: Vec<File>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
struct File {
    length: u32,       // length of the file in bytes
    path: Vec<String>, // list of UTF-8 encoded strings corresponding to subdirectory names, the last element is the file name
}

/// Parses the .torrent file
/// unsafe because it modifies a static variable
pub fn parse_torrent_file(filename: &str) {
    let contents = read(filename).expect("invalid .torrent filename");
    let torrent =
        from_bytes::<Torrent>(contents.as_slice()).expect("bruv debnencoding the .torrent failed");

    //in the morning ill figure out if this is actually pulling the right object, this mayu be getting the external struct, so ill need to recurse on it till i find *another* struct, and return that
    let mut decoder = Decoder::new(contents.as_slice());
    let infodata = 'outer: loop {
        match decoder.next_object() {
            Ok(Some(Object::Dict(mut d))) => loop {
                match d.next_pair() {
                    Ok(Some((b"info", Object::Dict(d)))) => {
                        break 'outer d.into_raw();
                    }
                    Ok(Some((_, _))) => (),
                    Ok(None) => break,
                    Err(e) => panic!("meow trying to gety/decode infohash failed: {}", e),
                }
            },
            _ => (),
        }
    }
    .expect("meow trying to gety/decode infohash failed");

    let mut hash = Sha1::new();
    hash.update(infodata);

    let torrent = Torrent {
        //info_hash: hash.finalize().to_vec(),
        info_hash: {let mut h = [0; 20]; hash.finalize_into(&mut h.into()); h},
        torrent_mode: {
            if torrent.info.files.len() > 0 {
                TorrentMode::MultipleFile
            } else {
                TorrentMode::SingleFile
            }
        },
        ..torrent
    };

    TORRENT.set(torrent).expect("Failed to set torrent");

    println!("announce: {}", TORRENT.get().unwrap().announce);
    println!("length: {}", TORRENT.get().unwrap().info.length);
    println!("name: {}", TORRENT.get().unwrap().info.name);
    println!("piece length: {}", TORRENT.get().unwrap().info.piece_length);
    println!("pieces vec: {:?}", TORRENT.get().unwrap().info.pieces);
    println!("infohash: {:?}", TORRENT.get().unwrap().info_hash);
}

/// 20 byte SHA1 hashvalue of the swarm
pub fn get_info_hash() -> [u8; 20] {//&'static Vec<u8> { // Tien causing chaos
    //&TORRENT.get().unwrap().info_hash
    TORRENT.get().unwrap().info_hash
}

/// url of the tracker
pub fn get_tracker_url() -> &'static String {
    &TORRENT.get().unwrap().announce
}

/// length of each piece in bytes
pub fn get_piece_length() -> u32 {
    TORRENT.get().unwrap().info.piece_length
}

/// number of pieces in the file
pub fn get_number_of_pieces() -> u32 {
    TORRENT.get().unwrap().info.pieces.len() as u32 / 20
}

/// vector of 20 byte SHA1 hashes of each piece
/// each hash is a vector of 20 bytes
pub fn get_pieces() -> &'static Vec<u8> {
    &TORRENT.get().unwrap().info.pieces
}

/// file length in bytes
pub fn get_file_length() -> u32 {
    TORRENT.get().unwrap().info.length
}

#[cfg(test)]
mod test {
    use rusty_fork::rusty_fork_test;
    rusty_fork_test! {
    #[test]
    fn test_parse_torrent_file() {
        super::parse_torrent_file("../artofwar.torrent");
        assert_eq!(
            super::TORRENT.get().unwrap().announce,
            "http://128.8.126.63:6969/announce"
        );
        assert_eq!(super::TORRENT.get().unwrap().info.length, 63371);
        assert_eq!(super::TORRENT.get().unwrap().info.name, "artofwar.txt");
        assert_eq!(super::TORRENT.get().unwrap().info.piece_length, 32768);
        assert_eq!(
            super::TORRENT.get().unwrap().info.pieces,
            hex::decode(
                "148C74D24BC89E9C7BC1EA97B354AA0DFAD7041BA7C239739231CC40A30879640C7C390BBEE8BFF8"
            )
            .unwrap()
        );
        assert_eq!(super::TORRENT.get().unwrap().info.files.len(), 0);
        assert_eq!(super::TORRENT.get().unwrap().info_hash.len(), 20);
        assert_eq!(
            super::TORRENT.get().unwrap().torrent_mode,
            super::TorrentMode::SingleFile
        );
        assert_eq!(
            super::TORRENT.get().unwrap().info_hash,
            hex::decode("a994e40f6c625f26834dfaafcb40d5c5f59fa648").unwrap()
        );
    }}

    rusty_fork_test! {
    #[test]
    fn test_all_the_accessors() {
        super::parse_torrent_file("../artofwar.torrent");
        assert_eq!(
            super::get_info_hash(),
            &hex::decode("a994e40f6c625f26834dfaafcb40d5c5f59fa648").unwrap()
        );
        assert_eq!(
            super::get_tracker_url(),
            &"http://128.8.126.63:6969/announce"
        );
        assert_eq!(super::get_piece_length(), 32768);
        assert_eq!(super::get_number_of_pieces(), 2);
        assert_eq!(
            super::get_pieces(),
            &hex::decode(
                "148C74D24BC89E9C7BC1EA97B354AA0DFAD7041BA7C239739231CC40A30879640C7C390BBEE8BFF8"
            )
            .unwrap()
        );
        assert_eq!(super::get_file_length(), 63371);
    }}

    rusty_fork_test! {
    #[test]
    fn test_multifile_parsing() {
        super::parse_torrent_file("../taytay.torrent");
        assert_eq!(
            super::TORRENT.get().unwrap().announce,
            "http://bigfoot1942.sektori.org:6969/announce"
        );
        assert_eq!(super::TORRENT.get().unwrap().info.length, 0);
        assert_eq!(super::TORRENT.get().unwrap().info.name, "Taylor Swift - Reputation (2017) (Mp3 320kbps) [Hunter]");
        assert_eq!(super::TORRENT.get().unwrap().info.piece_length, 1048576);
        assert_eq!(
            super::TORRENT.get().unwrap().info.pieces,
            hex::decode(
                "460A5F6DE49C2A46DB7D1603DA80F4A8D3206C5F2B45345C690846A9951EFF8A7C8F77688557303FD877E4832A1A2786D38384AF6649DCC955804C40B6DF084F4EFFA09CA4DB353F0DF3B1188CDA279ABE34B387FAEFE8C3676717E818809041FD316F8227DB7CDB0753199E414597BADB7C52EEBDD7FDC0BDD520165CB90772DA71650E41B1D5D6EE2910B9A5059800934BDF19EE9B0FEB309AA880C9CBD6D621C7741C903D98876820BD681105048C473C621788DF45B1B6FE475454EB6783EED1E2E93D652D209D13D7193098E7D30A704AE9173A7BEDFCA9B3C422CB5387D151FC7FC8862051BF43E6721FCE89DE1E9AD98F3FE3C56919B3F355D5FB3F6AE56DCCC102A93BB9C2F327CA8EE3E364341D16E3DD5DA3632BDE8B39D7757E30850E677B3D930D5757C2C10FFFE99E51ABE3272E70207A2D59A236D9B5EE349AFA6793B545635EF36F1C03C944FFA075E267D8D978710A3B3B1DE3DEB12A1644568861425A897960EF1E06FD83F0DF622D87E28C0E9E7107CA3E2448A14E835ADA79EFAFDB4AA8456A50269A83CB749490FEBBA044DE2162DB65EE86512381C3229419CF6B60EE62444EE996C5ACDA10CF2B7804F1659F227E030D8308ABEE134B81C8F7D7EDF4270421D5D3ADB4562010DF0F7A09E7556A8B684F35492CFE0931E569AA77D44E0BFF8AEB595A889482D64B0280CA3D198266710DB19A1AD6EC1EB71B6002F7134EC6E6B257652B072D4C38BA304FCC92B335D5E97E10B3AE89148CC746A20A2786DA16CEF1D9E4EAB6B4E8D2861E863ED2453E22A632776BBF206D5AF3A9BEF3AEAD9DEC37997A07B227C839B82C868D57A7D81E83438664599DA308ADF7BA2F459F0CAF732EACCAFC0E60A81BF3314B665DE308A4290A0990C40A82032193809FD1ACFDA7E3981A749E7AE4DA1C3BC7F6CACD9914CCFC0B968F6F29378622ACAF2AE02F1B9678DB6FFEDABAE14AF3D481B2D2A10BA98C1E81CC777BF4FB65FB7184F63AC659D620DD7EF6A696C5A344DB1FAA8B55FABA0BC55A090AFED558DAB6E31AA55DA6CDA6CEBACFD074DEB3D7FDCF9A2E555565FB02CEC5807EF9CE5114125F3FAC62545EB8F361587A3D2BB847ABD682E6581AB18A09D795F504DC9D7DE05DD9B13582441536D13829ACAA4405A14750D4E56C72EF32760ACA5F04476406BF54CD8E972B9CCD2B5E524D63EDFD3071282DCF2AC1E9C4D700060208F25A997C117677AC41F0A230E440A5BFFC599B54ECDD5F986531817E22581C78FC79C521385BB9B52EE2E5453951CFCC238CBDFD1CEC6917F72E90B9A576560F9A0B37D1BCCE70C51676771F0285C7F76E6C3801B90702F27B58322BBFA5C504187FA6201DB94F726841C6D4E6825B4D66D0E275BB4B7B818AD5EF035654601F56A3B99E52464346F63722901F752E3B8142E2C375C79AF9116900CDADA2401CA4502BAF754D7ADCE7AE91D6DF0A88FD9ED33DD3BAC1F4307B8180564170D2F78E1F15FDB603017EF0B7AD532BCB41C38150E567CA2E0D4C6FCC28C7F68D2E11CB9A41029936B328A3A45933522F876E19773052420B8BBB530025E2565A532B9E7950CD0F31B574958B37A909297EBD9E95DD4F320843EB4E02777D0AAFDF7C658884191E4730BD4688534B7550ECA7CC628BB3348E1E6060A8A1686B7B04C6EED0117148FAC87FAB8F3775ABCAD85F231CBB4133ED8560F0E6D7E32EC5CBDB4DA05CA02D2A666840B0308998B7C2224E4A099ACDCDD1553DA8AD03C5C6C1B7FD473FA408BC575791E3A0DFE57B5141C9B3FA8EC19B7E80A50E64F7E1606394C37B111265D12E8E76257E388061EB0FC026615F4DF359DF1AB71C5F134F2F9C80634B079E1723F983507F77306A8B22DA9CAC5DCC311CF109A0DF3D96E8B27D150ECA4C0277C0DBE55A54115C83EFFB75E20BB4FC9A7CB29113EEB91C3CCA5741DD605D920EE0FF81AFD3F1FAFAB96C1A57496BA7BBE237D1CB2B938D585720713EF2C8E78DB940E4EAF228E4ACCEFE31C620B903107988A21A48F3DCCFCC2BFB31B2B4074318D0149933A8FC39D13F975833F4AED6658CFA1B9680FC2F306F6D6D724357B0F1BAF335309CCDD21F8557D8527F87073C5469842A4EBAA39AEC004EAE2523BF029612B43B57A20E7251952BFB4FF7BB485D702EB8EF92D06CEE7C910C449BBB2C6B22EF99D3A1291D7F28C782272AEA8328745D0D7C639509E1711D2C111D1B035650C56850436151B0E751A3CAA2C9419555778063010B4ED5C9ABE6E00DB36D95C235CEC7EBA56F05BDA7DCDAD978676450C33B398889C0D947D78C3033950ABC0B6AF0F12168772653166073C2B1D196DE127761A09A7137762D6E56723F6968E5B188354D1D0D0E1C9214C2D0D44E6CEF6C3CC0373BCC53C8B8F516387CA40FA1DC10E633FA357994E7B7A869E28547C7609D0F253960075F9D5723A4D6500AF27CFFE5D0C14E24D8E6F28AA3174755168516953738A88989759121ADBB86D4FF98EF3545B10F819F43D86228B48066050B2DA2DB476CED4629CC3BAEF9B834CCE08B71BA2B8D714265F1DA053494C041B7129B20FD3A0FA15E4F2B53B58EC6042E284166D7E5663E23C8C8CEB27F674E7A87ED04A5C659FA167ABEC4B234C429ACD87D99034521E9D617E6E2A82F0682C24EB054693FB0F493B5265BE49071CC296F69F929F22932307FE44005F35B7257BCC60AF6ABCF9DACE7AEF9BB989E24BA01145618E4064EB555E379890555F7C659D54AB89182E03A921CBD54EA1502C91591A2B8DC01C58A4CDC19C0FF90D0264C87FA4D934EEBD08C8E1B5FB0E5286E1396CE2BBB8DE83B670CCDE2C4B5EC76E094C844E017852040A4DAD78B9EFD0CD74E1DD2847AC96B19971F1C33A9E331684C289087767873291E7E274916C4B26841E0212D7EA050A9B4ECEDE35C3ADDD79A69C2904A896AA5262379893562CDFECD6DE2E6027EBF12916D7BEA5D47F5F24596913F5F5628104E3337012B1CFBE9FBB2951DEE7AFAF0BB040F3D5DD94D97C58655BB9DEA76B9F4FE533AD3E642DD7F8F5777BC006955C8E90FB3684CDDBAC9E679B081E7259ED96252E68E219D90D5407B47A08E3EAC5DF8BFAC79939D947523E410EC93EB2351A28B99C9AAE23BCA041B122A43D91C8FB6C5F978AC4B1AC6B8986F5FDCD9A1275F4525281D31B6F94A0DEBFF8961BB64B3EA7ADDC401140EDB7F992D6001CE1AE5A360B7AFBDCEE7EF041F560AD73669B7F2087348CEA5F901AC6CDA02C130DF08651CA419B3DD4D2079D8FCE95FC2284D7C97BE6008A15B0EBB172AF63BA540C620A7EA8BFB72929FEBED744013DD0E6EC6CFBA02EA6FA02888ACC943592B9FFB05CDA8C93F3A65653F16DFDA97A1F41D48FF00A38AE7D3D2991E927FFCEED312D5C1CD95B953DAC30235E1F1BDC4FE56C31F7ECE4BE5E49270A07F4C798BF803B438652FDE8ED2B3B7D2BB8CFFEDBBA608BC5349652CEB23435C1F25B91690FCF76CE3B38980574D83D6DCD735E31510C367A9850EDDC024C5397ED1C775550D5BBB6A63A825253E3286A9A98129D2CF28C162950F1182A91DD6CEB647108DFFF7297F81CEC432CBBC871566FB5406659C6728D1F46F44A9842F9C8191AEB21A2DF7D91D8DA9840829D8B8CD5B502261007E0E277FE564A474AB665464"
            )
            .unwrap()
        );
        assert_eq!(super::TORRENT.get().unwrap().info.files.len(), 17);
        assert_eq!(super::TORRENT.get().unwrap().info_hash.len(), 20);
        assert_eq!(
            super::TORRENT.get().unwrap().torrent_mode,
            super::TorrentMode::MultipleFile
        );
        assert_eq!(
            super::TORRENT.get().unwrap().info_hash,
            hex::decode("1BC26A2B6ACA36BBD9A1E637DD085D6E38CFE4C0").unwrap()
        );
        assert_eq!(
            super::TORRENT.get().unwrap().info.files[0].length,
            8425596
        );
        assert_eq!(
            super::TORRENT.get().unwrap().info.files[0].path,
            vec![
                "Taylor Swift - Reputation (2017)",
           "1  ...Ready For It_.mp3"
            ]
        );
        assert_eq!(
            super::TORRENT.get().unwrap().info.files[16].length,
            416869
        );
        assert_eq!(
            super::TORRENT.get().unwrap().info.files[16].path,
            vec![
                "Taylor Swift - Reputation (2017)",
                "Music Zone.jpg"
            ]
        )
    }}
}
