// C:\Users\hrag\Sync\Programming\cfbf_cdf_ole_format_filetype_notes.md
// 

use std::{collections::HashMap, fs::File, io::{Read, Seek, SeekFrom}, ops::Deref, path::Path};

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use helper_lib::{datetime::windows_filetime_to_utc, strings::{string_from_utf16_as_vec_u8, vec_u8_as_hex}};
use log::*;
use num_enum::TryFromPrimitive;

mod property_definitions;
use property_definitions::{PropId,PropType};

// #[derive(Debug)]
// #[allow(non_snake_case, dead_code)]
// struct Root {
//     dwReserved: u32,
//     ibFileEof: u64,   // Logical EOF
//     ibAMapLast: u64,  // Last allocation map
//     cbAMapFree: u64,  // Free space in AMaps
//     cbPMapFree: u64,  // Free space in PMaps
//     BREFNBT: Bref,    // Node B-tree root
//     BREFBBT: Bref,    // Block B-tree root
//     fAMapValid: u8,
//     // bReserved: [u8; 3],
//     // wReserved: u32,
// }

#[derive(Debug)]
#[allow(dead_code)]
struct Bref {
    bid: u64,
    ib: u64,
}

#[derive(Debug)]
#[allow(dead_code)]
struct BlockInfo {
    offset: u64,
    size: usize,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Node {
    nid_type: u8,
    data_bid: u64,
    sub_bid: u64,
    parent: u32,
    sub_nodes:HashMap<u32, Node>,
}

#[derive(Debug, PartialEq)]
enum BlockType {
    Raw,
    HeapOnNode,
    XBlock,
    XXBlock,
}

#[repr(u8)]
#[derive(Debug, PartialEq, TryFromPrimitive)]
enum RecipientType {
    To = 0x01,
    Cc = 0x02,
    Bcc = 0x03,
}

#[derive(Debug)]
#[allow(dead_code)]
struct HNBlock {
    index: usize, //This index gives the type of block. First is header for next 8, then HNBITMAPHDR every 128 blocks (see https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/822e2327-b29d-4ec4-91be-45637a438d40)
    //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/8e4ae05c-3c24-4103-b7e5-ffef6f244834
    //HNHDR
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/8e4ae05c-3c24-4103-b7e5-ffef6f244834
    ib_hnpm: u16,
    b_sig: u8,
    b_client_sig: u8,
    hid_user_root: Hid,
    rgb_fill_level: u32,
    //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/291653c0-b347-4c5b-ba41-85ad780b4ba4
    c_alloc: u16,
    c_free: u16,
    //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/2dd1a95a-c8b1-4ac5-87d1-10cb8de64053
    bth_chunks: Vec<Vec<u8>>,
}

// struct XBLOCK {
//     // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/5b7a6935-e83d-4917-9f62-6ce3707f09e0
//     btype: u8,
//     c_level: u8,
//     c_ent: u16,
//     lcb_total: u32,
//     rgbid: Vec<BlockInfo>,
//     hn_blocks: Vec<HNBlock>,
//     //BLOCKTRAILER https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/a14943ef-70c2-403f-898c-5bc3747117e1
// }

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
struct Hid {
    hid: u32,
    hid_type: u8,
    hid_index: u16,
    hid_block_index: u16,
}
impl Hid {
    fn new(i: u32) -> Self {
        // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/85b9e985-ea53-447f-b70c-eb82bfbdcbc9
        // let hid_type = ((i >> 27) & 0x1F) as u8;        // top 5 bits
        // let hid_index = ((i >> 16) & 0x7FF) as u16;     // next 11 bits
        // let hid_block_index = (i & 0xFFFF) as u16;      // last 16 bits
        let hid_type = (i & 0x1F) as u8;               // last 5 bits
        let hid_index = ((i >> 5) & 0x7FF) as u16;     // next 11 bits
        let hid_block_index = (i >> 16) as u16;        // top 16 bits
        Hid {
            hid: i,
            hid_type,
            hid_index,
            hid_block_index,
        }
    }
}

// // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/36c1290e-8b1b-4d8c-91e1-d9fb3147c11c
// #[derive(Debug)]
// #[allow(dead_code)]
// struct PropertyInfo {
//     prop_id: u16,
//     prop_type: PropType,
//     prop_name: String,
// }

#[derive(Debug)]
#[allow(dead_code)]
struct PropertyEntry {
    property: Property,
    prop_value: [u8;4],
    value_string: String,
    value_bool: Option<bool>,
    value_i16: Option<i16>,
    value_i32: Option<i32>,
    value_i64: Option<i64>,
    value_f32: Option<f32>,
    value_f64: Option<f64>,
    value_datetime: Option<DateTime<Utc>>,
    value_binary: Option<Vec<u8>>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct ColumnEntry {
    tag: u32,
    data_offset: u16,
    data_size: u8,
    ceb_bit: u8,
    property: Property,
    value_string: Option<Vec<String>>,
    value_bool: Option<Vec<bool>>,
    value_i16: Option<Vec<i16>>,
    value_i32: Option<Vec<i32>>,
    value_i64: Option<Vec<i64>>,
    value_f32: Option<Vec<f32>>,
    value_f64: Option<Vec<f64>>,
    value_datetime: Option<Vec<DateTime<Utc>>>,
    value_binary: Option<Vec<Vec<u8>>>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct TableRows {
    num_rows: usize,
    rows: HashMap<PropId, ColumnEntry>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Property {
    prop_id: PropId,
    prop_type: PropType,
}
impl Property {
    fn new(prop_tag: [u8;4]) -> Self {
        let prop_id = u16::from_le_bytes([prop_tag[0], prop_tag[1]]);
        let prop_type = u16::from_le_bytes([prop_tag[2], prop_tag[3]]);
        Property {
            prop_id: PropId::try_from(prop_id).unwrap_or(PropId::Unknown),
            prop_type: PropType::try_from(prop_type).unwrap_or(PropType::Unknown),
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct Recipient {
    display_name: String,
    email_address: String,
    recipient_type: RecipientType,
}

// https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/5faf4800-645d-49d1-9457-2ac40eb467bd
const MPBB_CRYPT: [u8; 768] = [
    65,54,19,98,168,33,110,187,244,22,204,4,127,100,232,93,30,242,203,42,116,197,94,53,210,149,71,158,150,45,154,136,76,125,132,63,219,172,49,182,72,95,246,196,216,57,139,231,35,59,56,142,200,193,223,37,177,32,165,70,96,78,156,251,170,211,86,81,69,124,85,0,7,201,43,157,133,155,9,160,143,173,179,15,99,171,137,75,215,167,21,90,113,102,66,191,38,74,107,152,250,234,119,83,178,112,5,44,253,89,58,134,126,206,6,235,130,120,87,199,141,67,175,180,28,212,91,205,226,233,39,79,195,8,114,128,207,176,239,245,40,109,190,48,77,52,146,213,14,60,34,50,229,228,249,159,194,209,10,129,18,225,238,145,131,118,227,151,230,97,138,23,121,164,183,220,144,122,92,140,2,166,202,105,222,80,26,17,147,185,82,135,88,252,237,29,55,73,27,106,224,41,51,153,189,108,217,148,243,64,84,111,240,198,115,184,214,62,101,24,68,31,221,103,16,241,12,25,236,174,3,161,20,123,169,11,255,248,163,192,162,1,247,46,188,36,104,117,13,254,186,47,181,208,218,61,20,83,15,86,179,200,122,156,235,101,72,23,22,21,159,2,204,84,124,131,0,13,12,11,162,98,168,118,219,217,237,199,197,164,220,172,133,116,214,208,167,155,174,154,150,113,102,195,99,153,184,221,115,146,142,132,125,165,94,209,93,147,177,87,81,80,128,137,82,148,79,78,10,107,188,141,127,110,71,70,65,64,68,1,17,203,3,63,247,244,225,169,143,60,58,249,251,240,25,48,130,9,46,201,157,160,134,73,238,111,77,109,196,45,129,52,37,135,27,136,170,252,6,161,18,56,253,76,66,114,100,19,55,36,106,117,119,67,255,230,180,75,54,92,228,216,53,61,69,185,44,236,183,49,43,41,7,104,163,14,105,123,24,158,33,57,190,40,26,91,120,245,35,202,42,176,175,62,254,4,140,231,229,152,50,149,211,246,74,232,166,234,233,243,213,47,112,32,242,31,5,103,173,85,16,206,205,227,39,59,218,186,215,194,38,212,145,29,210,28,34,51,248,250,241,90,239,207,144,182,139,181,189,192,191,8,151,30,108,226,97,224,198,193,89,171,187,88,222,95,223,96,121,126,178,138,71,241,180,230,11,106,114,72,133,78,158,235,226,248,148,83,224,187,160,2,232,90,9,171,219,227,186,198,124,195,16,221,57,5,150,48,245,55,96,130,140,201,19,74,107,29,243,251,143,38,151,202,145,23,1,196,50,45,110,49,149,255,217,35,209,0,94,121,220,68,59,26,40,197,97,87,32,144,61,131,185,67,190,103,210,70,66,118,192,109,91,126,178,15,22,41,60,169,3,84,13,218,93,223,246,183,199,98,205,141,6,211,105,92,134,214,20,247,165,102,117,172,177,233,69,33,112,12,135,159,116,164,34,76,111,191,31,86,170,46,179,120,51,80,176,163,146,188,207,25,28,167,99,203,30,77,62,75,27,155,79,231,240,238,173,58,181,89,4,234,64,85,37,81,229,122,137,56,104,82,123,252,39,174,215,189,250,7,244,204,142,95,239,53,156,132,43,21,213,119,52,73,182,18,10,127,113,136,253,157,24,65,125,147,216,88,44,206,254,36,175,222,184,54,200,161,128,166,153,152,168,47,14,129,101,115,228,194,162,138,212,225,17,208,8,139,42,242,237,154,100,63,193,108,249,236
];

fn main() -> Result<()> {
    helper_lib::setup_logger(LevelFilter::Debug, None, "", "html5ever");
    
    info!("start");

    println!("Hello, world!");

    let pst_path = Path::new(r"C:\Users\hrag\OutlookData\test.pst");
    // let pst_path = Path::new(r"C:\Users\hrag\OutlookData\2013.pst");

    let mut bbt_map: HashMap<u64, BlockInfo> = HashMap::new();
    let mut nbt_map: HashMap<u32, Node> = HashMap::new();

    let mut file = File::open(pst_path)?;

    file.seek(SeekFrom::Start(0))?;
    let mut buffer = vec![0u8; 4];
    let _ = file.read_exact(&mut buffer)?;

    // println!("Read {} bytes", bytes_read);
    // println!("{}", vec_u8_as_hex(&buffer, true, " "));
    // println!("{}", String::from_utf8_lossy(&buffer));
    // println!("{:?}", buffer);
    // println!("{:?}", check_magic_bytes(&file));
    if !check_magic_bytes(&file) {
        bail!("File is not a pst !BDN file")
    }

    let w_ver = get_u16(&file, 0x0A)?;
    if w_ver!=23 {
        bail!("Unexpected pst version: {}", w_ver)
    }
    let w_ver_client = get_u16(&file, 0x0C)?;
    if w_ver_client!=19 {
        bail!("Unexpected w_ver_client version: {}", w_ver_client)
    }
    
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/32ce8c94-4757-46c8-a169-3fd21abee584
    let brefnbt = Bref {
        bid: get_bid(&file, 0xD8)?,
        ib: get_u64(&file, 0xE0)?,
    };
    let brefbbt = Bref {
        bid: get_bid(&file, 0xE8)?,
        ib: get_u64(&file, 0xF0)?,
    };
    // let root = Root {
    //     dwReserved: get_u32(&file, 0xB4)?,
    //     ibFileEof: get_u64(&file, 0xB8)?,
    //     ibAMapLast: get_u64(&file, 0xC0)?,
    //     cbAMapFree: get_u64(&file, 0xC8)?,
    //     cbPMapFree: get_u64(&file, 0xD0)?,
    //     BREFNBT: brefnbt,
    //     BREFBBT: brefbbt,
    //     fAMapValid: get_u8(&file, 0xF8)?,
    // };
    let b_crypt_method = get_u8(&file, 0x0201)?;
    let bid_next_b = get_u64(&file, 0x0204)?;

/*
Value	Friendly name	        Meaning
0x00	NDB_CRYPT_NONE	        Data blocks are not encoded.
0x01	NDB_CRYPT_PERMUTE	    Encoded with the Permutation algorithm (section 5.1).
0x02	NDB_CRYPT_CYCLIC	    Encoded with the Cyclic algorithm (section 5.2).
0x10	NDB_CRYPT_EDPCRYPTED	Encrypted with Windows Information Protection.
*/
    if b_crypt_method > 1 {
        bail!("encryption method {} is not handled", b_crypt_method)
    }

    // println!("{:#?}", brefnbt);
    // println!("{:#?}", brefbbt);
    // println!("bid_next_b: {}", bid_next_b);
    // println!("b_crypt_method: {}", b_crypt_method);

    read_bt_entry(&mut file, brefbbt, &mut bbt_map, &mut nbt_map)?;
    // println!("{:#?}", bbt_map);
    read_bt_entry(&mut file, brefnbt, &mut bbt_map, &mut nbt_map)?;
    // println!("{:#?}", nbt_map);

    for (nid, node) in &nbt_map {
        // println!("{}: {:?}", nid, node);
        // println!("{:02X}", node.nid_type);
        if node.data_bid > 0 {
            let block_info = bbt_map.get(&node.data_bid).expect("There should always be a bbt entry");
            let mut block_data = get_block_data(&mut file, &block_info, false)?;
            // println!("{}: {:?}, {:?}", nid, node, bbt);
            // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/18d7644e-cb33-4e11-95c0-34d8a84fbff6
            if node.nid_type==0x02 { //NID_TYPE_NORMAL_FOLDER
                // let property_entries = get_properties(None, &mut block_data, &node, &b_crypt_method, &mut file, &bbt_map)?;
                // // println!("{:#?}", property_entries);
                // for propery_entry in property_entries {
                //     println!("  {:?} ({:?}): {}", propery_entry.prop_id, propery_entry.prop_type, propery_entry.value_string)
                // }
            } else if node.nid_type==0x04 { // NID_TYPE_NORMAL_MESSAGE
                // println!();
                // println!("nid#{}: {:?}, {:?}", nid, node, block_info);
                //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/a9c1981d-d1ea-457c-b39e-dc7fb0eb95d4
                //Blocks are assigned in sizes that are multiples of 64 bytes
                // if block_info.size % 64 != 0 {
                //     bail!("Blocks are assigned in sizes that are multiples of 64 bytes")
                // }
                
                // let block_data = get_block_data(&mut file, &block_info, true)?;
                // let block_trailer = &block_data[block_data.len()-16..block_data.len()];
                // println!("{}", vec_u8_as_hex(&block_trailer, true, " "));
                // let cb = u16::from_le_bytes(block_trailer[0..2].try_into().unwrap());
                // let w_sig = u16::from_le_bytes(block_trailer[2..4].try_into().unwrap());
                // let dw_crc = u32::from_le_bytes(block_trailer[4..8].try_into().unwrap());
                // let bid = u64::from_le_bytes(block_trailer[8..16].try_into().unwrap());
                // println!("block_trailer: {}, {}, {}, {}, {}, {}", block_info.size, block_data.len(), cb, w_sig, dw_crc, bid);

                // let mut block_data = get_block_data(&mut file, &block_info, false)?;
                // // println!("{}, {}", block_info.size, block_data.len());
                // // println!("{}", vec_u8_as_hex(&block_data, true, " "));
                // // println!("{}", String::from_utf8_lossy(&block_data));

                // let prop_ids = Some(vec![PropId::DisplayName]);
                // let property_entries = get_properties(prop_ids, &mut block_data, &node, &b_crypt_method, &mut file, &bbt_map)?;
                // // // println!("{:#?}", property_entries);
                // for propery_entry in property_entries {
                //     println!("  {:?} ({:?}): {}", propery_entry.prop_id, propery_entry.prop_type, propery_entry.value_string)
                // }

                let x = get_recipients(node, &mut file, &bbt_map, &b_crypt_method)?;
                break

            } else if node.nid_type==0x11 { //17 NID_TYPE_ATTACHMENT_TABLE
                // println!("{}: {:?}", nid, node);
            } else if node.nid_type==0x05 { //NID_TYPE_ATTACHMENT
                // let property_entries = get_properties(None, &mut block_data, &node, &b_crypt_method, &mut file, &bbt_map)?;
                // // println!("{:#?}", property_entries);
                // for propery_entry in property_entries {
                //     println!("  {:?} ({:?}): {}", propery_entry.prop_id, propery_entry.prop_type, propery_entry.value_string)
                // }
            } else if node.nid_type==0x12 { //18 NID_TYPE_RECIPIENT_TABLE

            }
        }
    }

    info!("end");

    Ok(())
}

fn get_column_entry_string(column_entries: &HashMap<PropId, ColumnEntry>, prop_id: PropId, irow: usize) -> Result<String> {
    match column_entries.get(&prop_id) {
        Some(column_entry) => {
            match column_entry.value_string.as_ref() {
                Some(rows) => {
                    Ok(rows[irow].clone())
                },
                None => {
                    bail!("no rows of type string in the column entry {:?}", column_entry)
                }
            }
        },
        None => {
            bail!("no column {:?} in column_entries", prop_id)
        }
    }
}

fn get_column_entry_i32(column_entries: &HashMap<PropId, ColumnEntry>, prop_id: PropId, irow: usize) -> Result<i32> {
    match column_entries.get(&prop_id) {
        Some(column_entry) => {
            match column_entry.value_i32.as_ref() {
                Some(rows) => {
                    Ok(rows[irow])
                },
                None => {
                    bail!("no rows of type i32 in the column entry {:?}", column_entry)
                }
            }
        },
        None => {
            bail!("no column {:?} in column_entries", prop_id)
        }
    }
}

fn get_recipients(node:&Node, file: &mut File, bbt_map: &HashMap<u64, BlockInfo>, b_crypt_method:&u8) -> Result<Vec<Recipient>> {
    let recipients_node = node.sub_nodes.iter().find(|n| n.1.nid_type==0x12);
    println!("{:?}", recipients_node);
    if let Some(recipients_node) = recipients_node {
        let node = recipients_node.1;
        let block_info = bbt_map.get(&node.data_bid).expect("There should always be a bbt entry");
        let mut block_data = get_block_data(file, &block_info, false)?;
        // println!("{}, {}", block_info.size, block_data.len());
        // println!("{}", vec_u8_as_hex(&block_data, true, " "));
        
        let prop_ids = Some(vec![PropId::DisplayName, PropId::SmtpAddress, PropId::RecipientType]);
        let table_rows = get_table(&prop_ids, &mut block_data, &node, &b_crypt_method, file, &bbt_map)?;
        // println!("{:#?}", table_rows);
        for irow in 0..table_rows.num_rows {
            let column_entries = &table_rows.rows;
            let display_name = get_column_entry_string(column_entries, PropId::DisplayName, irow)?;
            let email_address = get_column_entry_string(column_entries, PropId::SmtpAddress, irow)?;
            let recipient_type = get_column_entry_i32(column_entries, PropId::RecipientType, irow)?;
            let recipient_type = RecipientType::try_from(recipient_type as u8).unwrap_or(RecipientType::To);
            let recipient = Recipient {
                display_name: display_name.clone(),
                email_address: email_address.clone(),
                recipient_type,
            };
            println!("{}: {:#?}", irow, recipient);
        }

        // let block_type = get_block_type(&block_data, b_crypt_method)?;
        // println!("block_type: {block_type:?}");


    }
    Ok(())
}

fn get_block_type(block_data:&Vec<u8>, b_crypt_method:&u8) -> Result<BlockType> {
    // Block data types:
    // Heap-on-Node (HN) (block_data[2]==0xEC)
    // XBLOCK (block_data[0]==0x01)
    // XXBLOCK (block_data[0]==0x02)
    // raw data
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/8f34ce81-7a04-4a31-ba48-e05543daa77f
    if block_data[0]==0x01 {
        if block_data[1]==0x01 {
            return Ok(BlockType::XBlock);
        } else if block_data[1]==0x02 {
            return Ok(BlockType::XXBlock);
        } else {
            bail!("Unexpected XBLOCK type (should be 1 (XBLOCK) or 2 (XXBLOCK))")
        }
    } else {
        if *b_crypt_method == 1 && block_data[2]==0xFF {
            //0xEC encrypted with method 1 is 0xFF
            return Ok(BlockType::HeapOnNode);
        } else  if block_data[2]==0xEC {
            return Ok(BlockType::HeapOnNode);
        } else {
            return Ok(BlockType::Raw);
        }
    }
}

fn get_table(prop_ids:&Option<Vec<PropId>>, block_data:&mut Vec<u8>, node:&Node, b_crypt_method:&u8, file: &mut File, bbt_map: &HashMap<u64, BlockInfo>) -> Result<TableRows> {
    let block_type = get_block_type(block_data, b_crypt_method)?;
        
    if block_type==BlockType::HeapOnNode || block_type==BlockType::XBlock {
        let hn_blocks: Vec<HNBlock> = get_hn_blocks(block_data, b_crypt_method, file, bbt_map)?;
        let b_client_sig = hn_blocks[0].b_client_sig;
        if b_client_sig == 0x7C { //bTypePC Property Context (PC/BTH)
            return Ok(get_table_context(prop_ids, &hn_blocks, node, bbt_map, file, b_crypt_method)?);
        } else {
            //ref https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/8e4ae05c-3c24-4103-b7e5-ffef6f244834
            bail!("get_table(): b_client_sig {:02X} not implemented", b_client_sig)
        }
    } else if block_type==BlockType::XXBlock {
        bail!("XXBlock not implemented")
    } else {
        // HNPAGEHDR https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/9c34ecf8-36bc-45a1-a2df-ee35c6dc840a
        let page_map_loc = u16::from_le_bytes([block_data[0], block_data[1]]);
        let page_map = &block_data[page_map_loc as usize..];
        let num_chunks = u16::from_le_bytes([page_map[0], page_map[1]]);
        for i in 0..num_chunks {
            let offset_start = u16::from_le_bytes([page_map[i as usize*2+4], page_map[i as usize*2+5]]) as usize;
            let offset_end = u16::from_le_bytes([page_map[i as usize*2+6], page_map[i as usize*2+7]]) as usize;
            // println!("{}: {}-{}", i, offset_start, offset_end);
            let chunk = &block_data[offset_start..offset_end];
            // println!("    {}", vec_u8_as_hex(&chunk[0..4], true, " "));
            // println!("{}", string_from_utf16_as_vec_u8(&chunk));
        }
        // println!("{}", vec_u8_as_hex(&block_data, true, " "));
        // println!("{}", string_from_utf16_as_vec_u8(&block_data[3..]));
        // println!("{}", block_data.len());
        // println!("{}", page_map_loc);
        bail!("block_type {:?} not programmed for", block_type)
    }
}

fn get_table_context(prop_ids:&Option<Vec<PropId>>, hn_blocks: &Vec<HNBlock>, node:&Node, bbt_map: &HashMap<u64, BlockInfo>, file: &mut File, b_crypt_method:&u8) -> Result<TableRows> {
    //table context
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/5e48be0d-a75a-4918-a277-50408ff96740
    // The underlying TC data is separated into 3 entries: a header with Column descriptors, a RowIndex (a nested BTH), and the actual table data (known as the Row Matrix).
    //TCINFO header https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/45b3a0c5-d6d6-4e02-aebf-13766ff693f0
    let hid = &hn_blocks[0].hid_user_root;
    let tcinfo = &hn_blocks[hid.hid_block_index as usize].bth_chunks[hid.hid_index as usize];
    // println!("tcinfo: {}", vec_u8_as_hex(&tcinfo, true, " "));
    let b_type = tcinfo[0];
    if b_type!=0x7C {
        bail!("btype {:02X} MUST be 0x7C bTypeTC", b_type)
    }
    let c_cols = tcinfo[1]; //Column count.
    // let rgib = &tcinfo[2..10];
    let rgib_0_4b = u16::from_le_bytes(tcinfo[2..4].try_into().unwrap()); //TCI_4b Ending offset of 8- and 4-byte data value group.
    let rgib_1_2b = u16::from_le_bytes(tcinfo[4..6].try_into().unwrap()); //TCI_2b Ending offset of 2-byte data value group.
    let rgib_2_1b = u16::from_le_bytes(tcinfo[6..8].try_into().unwrap()); //TCI_1b Ending offset of 1-byte data value group.
    let rgib_3_bm = u16::from_le_bytes(tcinfo[8..10].try_into().unwrap()); //TCI_bm Ending offset of the Cell Existence Block.
    let hid_row_index = Hid::new(u32::from_le_bytes(tcinfo[10..14].try_into().unwrap()));
    let hnid_rows = Hid::new(u32::from_le_bytes(tcinfo[14..18].try_into().unwrap()));
    // let hid_index = &tcinfo[18..22]; //Deprecated. Ignore.
    let rg_tcoldesc = &tcinfo[22..];
    // println!("tcinfo: {:02X}, {}, ({},{},{},{}), {:?}, {:?}, {}", b_type, c_cols, rgib_0_4b,rgib_1_2b,rgib_2_1b,rgib_3_bm, hid_row_index, hnid_rows, rg_tcoldesc.len());
    // println!("rg_tcoldesc: {}\n{}", rg_tcoldesc.len(), vec_u8_as_hex(&rg_tcoldesc, true, " "));

    //TCOLDESC https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/3a2f63cf-bb40-4559-910c-e55ec43d9cbb
    let mut prev_tag:u32 = 0;
    //let mut data_cols:Vec<ColumnEntry> = Vec::new();
    let mut data_cols:HashMap<PropId, ColumnEntry> = HashMap::new();
    for i in 0..c_cols as usize {
    // for chunk in rg_tcoldesc.chunks_exact(8) {
        let chunk = &rg_tcoldesc[i*8..i*8+8];
        let property = Property::new([chunk[2],chunk[3],chunk[0],chunk[1]]);
        let tag = u32::from_le_bytes(chunk[0..4].try_into().unwrap());
        if property.prop_id!=PropId::Unknown && property.prop_type!=PropType::Unknown {
            // println!("property: {}, {:?}", vec_u8_as_hex(&chunk[0..4], true, " "), property);
            let ib_data = u16::from_le_bytes(chunk[4..6].try_into().unwrap()); //Data Offset
            let cb_data = chunk[6]; //Data size
            let i_bit = chunk[7];
            // println!("  tcoldesc: {}: {},{},{},{}, {:?}", i, tag, ib_data, cb_data, i_bit, property);
            if prop_ids.is_none() || prop_ids.as_ref().unwrap().contains(&property.prop_id) {
                let mut value_string: Option<Vec<String>> = None;
                let mut value_bool: Option<Vec<bool>> = None;
                let mut value_i16: Option<Vec<i16>> = None;
                let mut value_i32: Option<Vec<i32>> = None;
                let mut value_i64: Option<Vec<i64>> = None;
                let mut value_f32: Option<Vec<f32>> = None;
                let mut value_f64: Option<Vec<f64>> = None;
                let mut value_datetime: Option<Vec<DateTime<Utc>>> = None;
                let mut value_binary: Option<Vec<Vec<u8>>> = None;
                match property.prop_type {
                    PropType::String => {value_string = Some(Vec::new())}
                    PropType::String8 => {value_string = Some(Vec::new())}
                    PropType::Boolean => {value_bool = Some(Vec::new())}
                    PropType::Integer16 => {value_i16 = Some(Vec::new())}
                    PropType::Integer32 => {value_i32 = Some(Vec::new())}
                    PropType::Integer64 => {value_i64 = Some(Vec::new())}
                    PropType::Floating32 => {value_f32 = Some(Vec::new())}
                    PropType::Floating64 => {value_f64 = Some(Vec::new())}
                    PropType::Time => {value_datetime = Some(Vec::new())}
                    PropType::Binary => {value_binary = Some(Vec::new())}
                    _ => {bail!("get_table_context(): TODO handle property type {:?}", property.prop_type)}
                }
                data_cols.insert(property.prop_id.clone(),
                    ColumnEntry { 
                        tag,
                        data_offset: ib_data,
                        data_size: cb_data,
                        ceb_bit: i_bit,
                        property,
                        value_string,
                        value_bool,
                        value_i16,
                        value_i32,
                        value_i64,
                        value_f32,
                        value_f64,
                        value_datetime,
                        value_binary,
                    }
                );
            }
        }
        if prev_tag > tag {
            bail!("The entries in this array MUST be sorted by the tag field of TCOLDESC. Ref: https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/45b3a0c5-d6d6-4e02-aebf-13766ff693f0");
        }
        prev_tag = tag;
    }

    //TCROWID https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/e20b5cf4-ea56-48b8-a8fa-e086c9b862ca
    let tcrowid = get_bytes_from_hid(hid_row_index.hid, &hn_blocks, node, bbt_map, file, b_crypt_method)?;
    // println!("tcrowid: {}", vec_u8_as_hex(&tcrowid, true, " "));
    let bth_header = tcrowid;
    let b_type = bth_header[0]; //MUST be bTypeBTH.
    assert_eq!(b_type, 0xB5); //b_type must be bTypeBTH (181, 0xB5)
    let hid = Hid::new(u32::from_le_bytes(bth_header[4..8].try_into().unwrap()));
    let tcrowid = get_bytes_from_hid(hid.hid, &hn_blocks, node, bbt_map, file, b_crypt_method)?;
    // println!("tcrowid: {}", vec_u8_as_hex(&tcrowid, true, " "));
    let num_rows = tcrowid.len() / 8;
    for irow in 0..num_rows {
        let dw_row_id = u32::from_le_bytes(tcrowid[irow*8+0..irow*8+4].try_into().unwrap());
        let dw_row_index = u32::from_le_bytes(tcrowid[irow*8+4..irow*8+8].try_into().unwrap());
        // println!("  tcrowid: {}: {}, {}", irow, dw_row_id, dw_row_index);
    }

    //Row Matrix //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/7f5ec68f-d4fd-404f-95c3-fe3495a034ec
    let rows = get_bytes_from_hid(hnid_rows.hid, &hn_blocks, node, bbt_map, file, b_crypt_method)?;
    // println!("rows: {} / {} = {}", rows.len(), rgib_3_bm, rows.len() / rgib_3_bm as usize);
    // let display_name_col = tccols.iter().find(|c| c.property.prop_id==PropId::DisplayName).unwrap();
    // let email_address_col = tccols.iter().find(|c| c.property.prop_id==PropId::SmtpAddress).unwrap();
    let sizeof_block = 8192;
    let sizeof_blocktrailer = 16;
    let block_datasize = sizeof_block - sizeof_blocktrailer;
    let rows_per_block = block_datasize / rgib_3_bm as usize;
    if rows_per_block < num_rows {
        bail!("TODO, handle when num_rows > rows_per_block: {} > {}", num_rows, rows_per_block)
    }
    for irow in 0..num_rows {
        let iblock = irow / rows_per_block;
        let irowinblock = irow % rows_per_block;
        let row = &rows[irow*rgib_3_bm as usize..irow*rgib_3_bm as usize+rgib_3_bm as usize];
        // println!("row: {}:\n{}", irow, vec_u8_as_hex(&row, true, " "));
        let rg_ceb = &row[rgib_2_1b as usize..rgib_3_bm as usize];
        for (_, col) in data_cols.iter_mut() {
            let mut value_string: String = String::new();
            let mut value_bool: bool = bool::default();
            let mut value_i16: i16 = 0;
            let mut value_i32: i32 = 0;
            let mut value_i64: i64 = 0;
            let mut value_f32: f32 = 0f32;
            let mut value_f64: f64 = 0f64;
            let mut value_datetime: DateTime<Utc> = DateTime::default();
            let mut value_binary: Vec<u8> = Vec::default();
            
            let f_ceb: bool = (rg_ceb[col.ceb_bit as usize / 8] & (1 << (7 - (col.ceb_bit % 8)))) != 0;

            if f_ceb {
                let mut prop_value = row[col.data_offset as usize..col.data_offset as usize + col.data_size as usize].to_vec();
                // println!("    {}; {}", vec_u8_as_hex(&prop_value, true, " "), string_from_utf16_as_vec_u8(&prop_value));
                if [PropType::String, PropType::String8,PropType::Integer64,PropType::Floating64,PropType::Binary,PropType::Time,PropType::Guid].contains(&col.property.prop_type) {
                    prop_value = get_bytes_from_hid(u32::from_le_bytes(prop_value.try_into().unwrap()), &hn_blocks, node, bbt_map, file, b_crypt_method)?;
                }
                match col.property.prop_type {
                    PropType::String => { value_string = string_from_utf16_as_vec_u8(&prop_value); }
                    PropType::String8 => { value_string = String::from_utf8_lossy(&prop_value).to_string(); }
                    PropType::Boolean => { value_bool = prop_value[0]==1; }
                    PropType::Integer16 => { value_i16 = i16::from_le_bytes(prop_value[0..2].try_into().unwrap()); }
                    PropType::Integer32 => { value_i32 = i32::from_le_bytes(prop_value.try_into().unwrap()); }
                    PropType::Integer64 => { value_i64 = i64::from_le_bytes(prop_value.try_into().unwrap()); }
                    PropType::Floating32 => { value_f32 = f32::from_le_bytes(prop_value.try_into().unwrap()); }
                    PropType::Floating64 => { value_f64 = f64::from_le_bytes(prop_value.as_slice().try_into().unwrap()); }
                    PropType::Time => { value_datetime = windows_filetime_to_utc(u64::from_le_bytes(prop_value.try_into().unwrap())); }
                    PropType::Binary => { value_binary = prop_value; }
                    _ => {bail!("get_table_context(): TODO handle property type {:?}", col.property.prop_type)}
                }
            }

            match col.property.prop_type {
                PropType::String => { col.value_string.as_mut().unwrap().push(value_string) }
                PropType::String8 => { col.value_string.as_mut().unwrap().push(value_string) }
                PropType::Boolean => { col.value_bool.as_mut().unwrap().push(value_bool) }
                PropType::Integer16 => { col.value_i16.as_mut().unwrap().push(value_i16) }
                PropType::Integer32 => { col.value_i32.as_mut().unwrap().push(value_i32) }
                PropType::Integer64 => { col.value_i64.as_mut().unwrap().push(value_i64) }
                PropType::Floating32 => { col.value_f32.as_mut().unwrap().push(value_f32) }
                PropType::Floating64 => { col.value_f64.as_mut().unwrap().push(value_f64) }
                PropType::Time => { col.value_datetime.as_mut().unwrap().push(value_datetime) }
                PropType::Binary => { col.value_binary.as_mut().unwrap().push(value_binary) }
                _ => {bail!("get_table_context(): TODO handle property type {:?}", col.property.prop_type)}
            }
        }
    }

    // println!("{:#?}", data_cols);

    return Ok(TableRows { num_rows: num_rows, rows: data_cols } );

}

fn get_hn_blocks(mut block_data:&mut Vec<u8>, b_crypt_method:&u8, file: &mut File, bbt_map: &HashMap<u64, BlockInfo>) -> Result<Vec<HNBlock>> {
    let block_type = get_block_type(block_data, b_crypt_method)?;
    if block_type==BlockType::HeapOnNode {
        if *b_crypt_method == 1 {
            //NDB_CRYPT_PERMUTE
            //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/5faf4800-645d-49d1-9457-2ac40eb467bd
            decode_permute(&mut block_data);
        }
        let hn_block = get_hn_block(block_data, 0)?;
        // println!("hn_block: {:?}", hn_block);
        return Ok(vec![hn_block]);
    } else if block_type==BlockType::XBlock {
        let xblock_blocks = get_xblock_blocks(file, bbt_map, block_data, b_crypt_method)?;
        let mut hn_blocks: Vec<HNBlock> = Vec::new();
        for (iblock, xblock_block) in xblock_blocks.iter().enumerate() {
            // let mut xblock_block = xblock_block;
            hn_blocks.push(get_hn_block(&mut xblock_block.to_vec(), iblock as usize)?);
        }
        return Ok(hn_blocks);
    } else if block_type==BlockType::XXBlock {
        bail!("XXBlock not implemented")
    } else {
        bail!("blocktype {:?} not implemented", block_type)
    }
}

fn get_object_properties(prop_ids:Option<Vec<PropId>>, block_data:&mut Vec<u8>, node:&Node, b_crypt_method:&u8, file: &mut File, bbt_map: &HashMap<u64, BlockInfo>) -> Result<(Vec<PropertyEntry>)> {
    // Block data types:
    // Heap-on-Node (HN) (block_data[2]==0xEC)
    // XBLOCK (block_data[0]==0x01)
    // XXBLOCK (block_data[0]==0x02)
    // raw data
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/8f34ce81-7a04-4a31-ba48-e05543daa77f
    let block_type = get_block_type(block_data, b_crypt_method)?;
    // println!("block_type: {block_type:?}");


/*
Data Block
└── Heap-on-Node (HN)
└── Property Context (PC)
└── B-tree (BTH)
    └── Properties (e.g. 0x3001 = name)
*/
        
    if block_type==BlockType::HeapOnNode || block_type==BlockType::XBlock {
        let hn_blocks: Vec<HNBlock> = get_hn_blocks(block_data, b_crypt_method, file, bbt_map)?;
        let b_client_sig = hn_blocks[0].b_client_sig;
        if b_client_sig == 0xBC { //bTypePC Property Context (PC/BTH)
            return Ok(process_heap_node(prop_ids, &hn_blocks, node, bbt_map, file, b_crypt_method)?);
        } else {
            //ref https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/8e4ae05c-3c24-4103-b7e5-ffef6f244834
            bail!("get_object_properties(): b_client_sig {:02X} not implemented", b_client_sig)
        }
    } else if block_type==BlockType::XXBlock {
        bail!("XXBlock not implemented")
    } else {
        // HNPAGEHDR https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/9c34ecf8-36bc-45a1-a2df-ee35c6dc840a
        let page_map_loc = u16::from_le_bytes([block_data[0], block_data[1]]);
        let page_map = &block_data[page_map_loc as usize..];
        let num_chunks = u16::from_le_bytes([page_map[0], page_map[1]]);
        for i in 0..num_chunks {
            let offset_start = u16::from_le_bytes([page_map[i as usize*2+4], page_map[i as usize*2+5]]) as usize;
            let offset_end = u16::from_le_bytes([page_map[i as usize*2+6], page_map[i as usize*2+7]]) as usize;
            // println!("{}: {}-{}", i, offset_start, offset_end);
            let chunk = &block_data[offset_start..offset_end];
            // println!("    {}", vec_u8_as_hex(&chunk[0..4], true, " "));
            // println!("{}", string_from_utf16_as_vec_u8(&chunk));
        }
        // println!("{}", vec_u8_as_hex(&block_data, true, " "));
        // println!("{}", string_from_utf16_as_vec_u8(&block_data[3..]));
        // println!("{}", block_data.len());
        // println!("{}", page_map_loc);
        bail!("block_type {:?} not programmed for", block_type)
    }
}

fn get_xblock_blocks(file: &mut File, bbt_map: &HashMap<u64, BlockInfo>, block_data:&Vec<u8>, b_crypt_method:&u8) -> Result<Vec<Vec<u8>>> {
    let mut xblock_blocks:Vec<Vec<u8>> = Vec::new();
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/5b7a6935-e83d-4917-9f62-6ce3707f09e0
    let b_type = block_data[0]; //Block type; MUST be set to 0x01 to indicate an XBLOCK or XXBLOCK.
    if b_type!=1 { bail!("xblock b_type MUST be set to 0x01") }
    let c_level = block_data[1]; //MUST be set to 0x01 to indicate an XBLOCK (0x02 for XXBLOCK)
    if c_level!=1 { bail!("xblock c_level MUST be set to 0x01") }
    let c_ent = u16::from_le_bytes([block_data[2], block_data[3]]); //The count of BID entries in the XBLOCK.
    //let lcb_total = u32::from_le_bytes(block_data[4..8].try_into().unwrap()); //Total count of bytes of all the external data stored in the data blocks referenced by XBLOCK
    let end_chunk_loc = 8 + c_ent as usize * 8;
    // println!("end_chunk_loc: {end_chunk_loc}");
    let rgbid = &block_data[8..end_chunk_loc];
    // println!("  XBLOCK: {}, {}, {}, {}, {}", b_type, c_level, c_ent, lcb_total, rgbid.len());
    //XBLOCKS are just a pile of 8-byte links to HNs https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/a3fa280c-eba3-434f-86e4-b95141b3c7b1
    //  sometimes they are a pile of unicode blocks, not HNs
    //should we concatenate blocks? sometimes
    //we should build up an array of blocks
    //should this be async? Maybe in a future version.
    // let block_infos: Vec<BlockInfo> = Vec::new();
    // let mut block_array: Vec<Vec<u8>> = Vec::new();
    for ibid in 0..c_ent {
        let bid = &rgbid[(ibid*8) as usize..(ibid*8+8) as usize];
        let bid = bid_from_u64(u64::from_le_bytes(bid.try_into().unwrap()));
        // println!("    {bid}");
        let block_info = bbt_map.get(&bid).expect("There should always be a bbt entry");
        // println!("    {block_info:?}");
        let mut block_data = get_block_data(file, &block_info, false)?;
        if *b_crypt_method == 1 {
            //NDB_CRYPT_PERMUTE
            //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/5faf4800-645d-49d1-9457-2ac40eb467bd
            decode_permute(&mut block_data);
        }
        xblock_blocks.push(block_data);
    }

    Ok(xblock_blocks)
}

fn get_hn_block(block_data:&mut Vec<u8>, index:usize) -> Result<HNBlock> {
    // if *b_crypt_method == 1 {
    //     //NDB_CRYPT_PERMUTE
    //     //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/5faf4800-645d-49d1-9457-2ac40eb467bd
    //     decode_permute(&mut block_data);
    // }
    // println!("block_data # {}: {}", index, vec_u8_as_hex(&block_data, true, " "));
    // println!("block_data # {}: {}", index, string_from_utf16_as_vec_u8(&block_data));
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/a3fa280c-eba3-434f-86e4-b95141b3c7b1
    let hnhdr = &block_data[0..12];
    let ib_hnpm: u16 = u16::from_le_bytes([hnhdr[0],hnhdr[1]]); //The byte offset to the HN page Map record (section 2.3.1.5), with respect to the beginning of the HNHDR structure.
    // println!("  ib_hnpm: {}", ib_hnpm);
    let mut b_sig: u8 = 0;
    let mut b_client_sig: u8 = 0;
    let mut hid_user_root = Hid::new(0);
    let mut rgb_fill_level: u32 = 0;
    let is_hn_bitmap = index > 7 && (index - 8) % 128 == 0; //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/822e2327-b29d-4ec4-91be-45637a438d40
    if is_hn_bitmap {
        //not worrying about these at the moment
    } else if index==0 {
        //HNHDR HN Header
        // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/8e4ae05c-3c24-4103-b7e5-ffef6f244834
        b_sig = hnhdr[2];
        b_client_sig = hnhdr[3];
        hid_user_root = Hid::new(u32::from_le_bytes(hnhdr[4..8].try_into().unwrap()));
        rgb_fill_level = u32::from_le_bytes(hnhdr[8..12].try_into().unwrap());
        println!("  {}, {:02X}, {:02X}, {:?}, {}", ib_hnpm, b_sig, b_client_sig, hid_user_root, rgb_fill_level);
        if b_sig != 0xEC {
            bail!("bSig (1 byte): Block signature; MUST be set to 0xEC to indicate an HN.")
        }
        assert_eq!(b_sig, 0xEC); // sanity check
        if b_client_sig == 0xBC { //bTypePC Property Context (PC/BTH)

        } else if b_client_sig == 0x7C { // bTypeTC Table Context (TC/HN)

        } else {
            //ref https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/8e4ae05c-3c24-4103-b7e5-ffef6f244834
            bail!("b_client_sig {:02X} not implemented", b_client_sig)
        }
    } else {
        //HNPAGEHDR
        //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/9c34ecf8-36bc-45a1-a2df-ee35c6dc840a
    }

    let mut c_alloc: u16 = 0; //number of allocations
    let mut c_free: u16 = 0; //number of allocations
    let mut bth_chunks:Vec<Vec<u8>> = Vec::new();
    if !is_hn_bitmap {
        //HN Page Map (HNPAGEMAP)
        // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/291653c0-b347-4c5b-ba41-85ad780b4ba4
        let hnp = &block_data[ib_hnpm as usize..];
        c_alloc = u16::from_le_bytes([hnp[0], hnp[1]]); //number of allocations
        c_free = u16::from_le_bytes([hnp[2], hnp[3]]); //number of allocations
        // println!("  c_alloc: {}", c_alloc);
        //The heap is divided into allocation chunks.
        //Parse allocations
        let mut offsets = Vec::new();
        // Important: there are cAlloc + 1 offsets
        for i in 0..=(c_alloc+1) as usize {
            let off = u16::from_le_bytes([
                hnp[2 + i*2],
                hnp[2 + i*2 + 1],
            ]);
            offsets.push(off as usize);
        }
        // println!("  offsets: {:?}", offsets);
        //offsets = array like [0, 12, 20, 76, 124], giving byte sections into chunks
        // first chunk is 12 bytes (0-12) HN header
        // second chunk is 8 bytes (12-20) BTH header
        // following chunks vary
        // 3rd Property entries
        // Property value (string)

        //the allocation has offset entries like this:
        for i in (0..offsets.len()-1) {
            let start = offsets[i];
            let end   = offsets[i + 1];

            let chunk = &block_data[start..end];
            // let mut print_chunk = chunk;
            // if print_chunk.len() > 20 {
            //     print_chunk = &print_chunk[0..20]
            // }
            // println!("{}:{}: {}, {}...", index, i, chunk.len(), vec_u8_as_hex(&print_chunk, true, " "));
            bth_chunks.push(chunk.to_vec());
        }
    }

    Ok(HNBlock {
        index,
        ib_hnpm,
        b_sig,
        b_client_sig,
        hid_user_root,
        rgb_fill_level,
        c_alloc,
        c_free,
        bth_chunks,
    })

}

// fn process_heap_node(block_data:&mut Vec<u8>) -> Result<()> {
fn process_heap_node(prop_ids:Option<Vec<PropId>>, hn_blocks: &Vec<HNBlock>, node:&Node, bbt_map: &HashMap<u64, BlockInfo>, file: &mut File, b_crypt_method:&u8) -> Result<(Vec<PropertyEntry>)> {
    // first hn provides property entries
    // HNHDR

    // BTHHEADER
    let bthheader_loc = 1;
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/5a6ab19e-1f44-4def-ad64-7bd82d94bd78
    let bth_header = &hn_blocks[0].bth_chunks[bthheader_loc]; //8 bytes
    // println!("bth_header: {}", vec_u8_as_hex(bth_header, true, " "));
    let b_type = bth_header[0]; //MUST be bTypeBTH.
    assert_eq!(b_type, 0xB5); //b_type must be bTypeBTH (181, 0xB5)
    let cb_key = bth_header[1]; //Size of the BTree Key value, in bytes. This value MUST be set to 2, 4, 8, or 16.
    if !vec![2,4,8,16].contains(&cb_key) {
        bail!("bth_header, cb_key={}, Size of the BTree Key value, in bytes. This value MUST be set to 2, 4, 8, or 16.", cb_key)
    }
    let cb_ent = bth_header[2]; //Size of the data value, in bytes. This MUST be greater than zero and less than or equal to 32.
    let bldx_levels = bth_header[3]; //Index depth. This number indicates how many levels of intermediate indices exist in the BTH. Note that this number is zero-based, meaning that a value of zero actually means that the BTH has one level of indices. If this value is greater than zero, then its value indicates how many intermediate index levels are present.
    let hid = Hid::new(u32::from_le_bytes(bth_header[4..8].try_into().unwrap()));
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/85b9e985-ea53-447f-b70c-eb82bfbdcbc9
    // println!("  bth_header: {}, {}, {}, {}, {:?}", b_type, cb_key, cb_ent, bldx_levels, hid);
    if hid.hid_type!=0 {
        bail!("hid.hid_type MUST be set to 0 (NID_TYPE_HID) to indicate a valid HID.")
    }
    if hid.hid_index==0 {
        bail!("hid.hid_index is a 1-based index, MUST NOT be zero.")
    }
    // for (i, bth_chunk) in hn_blocks[0].bth_chunks.iter().enumerate() {
    //     println!("  {}: {}", i, vec_u8_as_hex(bth_chunk, true, " "));
    // }

    if bldx_levels != 0 {
        //bldx_levels==0 then this is a leaf node
        bail!("bth_header, bldx_levels={}, not programmed for >1 level yet.", bldx_levels)
        // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/2c992ac1-1b21-4167-b111-f76cf609005f
    }

    //Instead of directly referencing memory, PST uses Heap IDs (HIDs).
    // index → which allocation
    // type  → what kind of object
    // let hid_index = (hid_root & 0xFFFF) as usize;
    // let hid = hd_user_root;
    // let hid_index = (hid & 0xFFFF) as usize;
    // let chunk = allocations[hid_index(hid)];
    
    let property_entries_block = &hn_blocks[hid.hid_block_index as usize].bth_chunks[hid.hid_index as usize];
    let mut property_entries: Vec<PropertyEntry> = Vec::new();
    // println!("property_entries_block: {}", vec_u8_as_hex(&property_entries_block, true, " "));

    for property_entry in property_entries_block.chunks_exact(8) {
        // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/7daab6f5-ce65-437e-80d5-1b1be4088bd3
        // println!("property_entry: {}", vec_u8_as_hex(&property_entry, true, " "));
        //cbKey (2 bytes) + value (cb_ent bytes) (variable, aligned)
        // let w_prop_id = u16::from_le_bytes([property_entry[0], property_entry[1]]);
        // let w_prop_type = u16::from_le_bytes([property_entry[2], property_entry[3]]);
        let property = Property::new(property_entry[0..4].try_into().unwrap());
        if property.prop_id!=PropId::Unknown
        && property.prop_type!=PropType::Unknown {
            if prop_ids.is_none() || prop_ids.as_ref().unwrap().contains(&property.prop_id) {
                let w_prop_value = &property_entry[4..8];
                // println!("  w_prop_id:{:04X}, w_prop_type:{:04X}, w_prop_value:{}", w_prop_id, w_prop_type, vec_u8_as_hex(&w_prop_value, true, " "));
                let property_entry = get_property_entry(property, w_prop_value.try_into().unwrap(), hn_blocks, node, bbt_map, file, b_crypt_method)?;
                property_entries.push(property_entry);
            }
        }
    }

    Ok(property_entries)

}

fn get_property_entry(property: Property, prop_value: [u8;4], hn_blocks: &Vec<HNBlock>, node:&Node, bbt_map: &HashMap<u64, BlockInfo>, file: &mut File, b_crypt_method:&u8) -> Result<PropertyEntry> {
    let value_string: String;
    let mut value_bool: Option<bool> = None;
    let mut value_i16: Option<i16> = None;
    let mut value_i32: Option<i32> = None;
    let mut value_i64: Option<i64> = None;
    let mut value_f32: Option<f32> = None;
    let mut value_f64: Option<f64> = None;
    let mut value_datetime: Option<DateTime<Utc>> = None;
    let mut value_binary: Option<Vec<u8>> = None;

    match property.prop_type {
        PropType::Null => { value_string = String::from("NULL") }
        PropType::Boolean => {
            value_bool = Some(prop_value[0]==1);
            if value_bool.unwrap() {
                value_string = String::from("True");
            } else {
                value_string = String::from("False");
            }
        }
        PropType::Integer16 => {
            let value = i16::from_le_bytes(prop_value[0..2].try_into().unwrap());
            value_string = value.to_string();
            value_i16 = Some(value);
        }
        PropType::Integer32 => {
            let value = i32::from_le_bytes(prop_value.try_into().unwrap());
            value_string = value.to_string();
            value_i32 = Some(value);
        }
        PropType::Integer64 => {
            let hid = u32::from_le_bytes(prop_value.try_into().unwrap());
            let value_bytes = get_bytes_from_hid(hid,hn_blocks,node,bbt_map,file,b_crypt_method)?;
            let value = i64::from_le_bytes(value_bytes.try_into().unwrap());
            value_string = value.to_string();
            value_i64 = Some(value);
        }
        PropType::Floating32  => {
            let value = f32::from_le_bytes(prop_value.try_into().unwrap());
            value_string = value.to_string();
            value_f32 = Some(value);
        }
        PropType::Floating64  => {
            let hid = u32::from_le_bytes(prop_value.try_into().unwrap());
            let value_bytes = get_bytes_from_hid(hid,hn_blocks,node,bbt_map,file,b_crypt_method)?;
            let value = f64::from_le_bytes(value_bytes.as_slice().try_into().unwrap());
            value_string = value.to_string();
            value_f64 = Some(value);
        }
        PropType::String => {
            let hid = u32::from_le_bytes(prop_value.try_into().unwrap());
            let value_bytes = get_bytes_from_hid(hid,hn_blocks,node,bbt_map,file,b_crypt_method)?;
            value_string = string_from_utf16_as_vec_u8(&value_bytes);
        }
        PropType::String8 => {
            let hid = u32::from_le_bytes(prop_value.try_into().unwrap());
            let value_bytes = get_bytes_from_hid(hid,hn_blocks,node,bbt_map,file,b_crypt_method)?;
            value_string = String::from_utf8_lossy(&value_bytes).to_string();
        }
        PropType::Binary => {
            let hid = u32::from_le_bytes(prop_value.try_into().unwrap());
            value_binary = Some(get_bytes_from_hid(hid,hn_blocks,node,bbt_map,file,b_crypt_method)?);
            value_string = String::new()
        }
        PropType::Time => {
            let hid = u32::from_le_bytes(prop_value.try_into().unwrap());
            let value_bytes = get_bytes_from_hid(hid,hn_blocks,node,bbt_map,file,b_crypt_method)?;
            let value = u64::from_le_bytes(value_bytes.try_into().unwrap());
            let value = windows_filetime_to_utc(value);
            value_string = value.to_string();
            value_datetime = Some(value);
        }
        PropType::Guid => {
            let hid = u32::from_le_bytes(prop_value.try_into().unwrap());
            let value_bytes = get_bytes_from_hid(hid,hn_blocks,node,bbt_map,file,b_crypt_method)?;
            value_string = vec_u8_as_hex(&value_bytes, true, " ");
        }
        _ => bail!("property type {:?} not implemented", property.prop_type)
    }

    Ok(PropertyEntry {
        property,
        prop_value,
        value_string,
        value_bool,
        value_i16,
        value_i32,
        value_i64,
        value_f32,
        value_f64,
        value_datetime,
        value_binary,
    })
}

fn get_bytes_from_hid(hid:u32, hn_blocks: &Vec<HNBlock>, node:&Node, bbt_map: &HashMap<u64, BlockInfo>, file: &mut File, b_crypt_method:&u8) -> Result<Vec<u8>> {
    if hid==0 {
        return Ok(Vec::new());
    }
    let hid = Hid::new(hid);

    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/85b9e985-ea53-447f-b70c-eb82bfbdcbc9
    // println!("    property_entry: {:04X}, {:04X}, {:?}", w_prop_id, w_prop_type, hid);
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/7ac490ce-31af-4a75-97df-eb9d07a003fd
    //    The HNID refers to an HID if the hidType is NID_TYPE_HID. Otherwise, the HNID refers to an NID.
    if hid.hid_type==0 {
        //HNID is a HID (<= 3580 bytes)
        if hid.hid_type!=0 { bail!("hid.hid_type MUST be set to 0 (NID_TYPE_HID) to indicate a valid HID.") }
        if hid.hid_index==0 { bail!("hid.hid_index is a 1-based index, MUST NOT be zero.") }
        let value = &hn_blocks[hid.hid_block_index as usize].bth_chunks[hid.hid_index as usize];
        return Ok(value.to_vec());
    } else {
        //HNID is a NID (subnode, > 3580 bytes)
        //  the NID is the local NID under the subnode where the raw data is located.
        //  Subnode BTree https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/0c7d9bd5-e3cf-43cc-9292-a32c7b2666da
        let sub_nid = hid;
        let value = get_sub_node_bytes(node, sub_nid, file, bbt_map, b_crypt_method)?;
        return Ok(value);
    }
}

fn get_sub_node_bytes(node:&Node, sub_nid:Hid, mut file: &mut File, bbt_map: &HashMap<u64, BlockInfo>, b_crypt_method:&u8) -> Result<Vec<u8>> {
    let mut value: Vec<u8> = Vec::new();

    //first 5 bytes are the nid type: https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/18d7644e-cb33-4e11-95c0-34d8a84fbff6
    // let sub_nid_type = (sub_nid & 0x1F) as u8;
    // nid_type    0x1F   NID_TYPE_LTP    LTP
    // println!("{:#?}", bbt_map);
    // println!("{:#?}", nbt_map);
    // println!("sub_nid: {sub_nid}, sub_nid_type: {sub_nid_type:02X}");
    // println!("{:?}", node);
    let sub_node = &node.sub_nodes.get(&sub_nid.hid).expect("Missing sub_nodes entry");
    // println!("{:?}", sub_node);
    // let node = nbt_map.get(&(hid.hid_index as u32)).expect("Missing nbt_map entry");
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/5182eb24-4b0b-4816-aa3f-719cc6e6b018
    let block_info = bbt_map.get(&sub_node.data_bid).expect("There should always be a bbt entry");
    let mut block_data = get_block_data(&mut file, &block_info, false)?;
    let block_type = get_block_type(&block_data, b_crypt_method)?;
    //assume an xblock of unicode data nodes
    if block_type==BlockType::XBlock {
        let xblock_blocks = get_xblock_blocks(file, bbt_map, &block_data, b_crypt_method)?;
        for xblock_block in xblock_blocks {
            // println!("{}", vec_u8_as_hex(&xblock_block, true, " "));
            value.extend_from_slice(&xblock_block);
        }
    } else if block_type==BlockType::Raw {
        if *b_crypt_method == 1 {
            decode_permute(&mut block_data);
        }
        value.append(&mut block_data);
        // println!("{}", vec_u8_as_hex(&block_data, true, " "));
        // println!("{}", string_from_utf16_as_vec_u8(&block_data));
    } else {
        bail!("sub_node blocks, unexpected block type: {:?}", block_type)
    }

    Ok(value)
}

fn get_block_data(file: &mut File, block_info: &BlockInfo, include_trailer:bool) -> Result<Vec<u8>> {
    //Blocks are assigned in sizes that are multiples of 64 bytes
    //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/ddeb714d-8fd5-4a48-8019-8338cb511c80
    let mut size = block_info.size;
    if include_trailer {
        size += 16; //add trailer bytes
        size += 64 - size % 64; //round up to nearest 64
    }
    file.seek(SeekFrom::Start(block_info.offset))?;
    let mut buf = vec![0; size];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

fn get_tables() -> (&'static [u8; 256], &'static [u8; 256]) {
    let r = MPBB_CRYPT[..256].try_into().unwrap();
    let i = MPBB_CRYPT[512..768].try_into().unwrap();
    (r, i)
}

fn decode_permute(data: &mut [u8]) {
    let (_, inverse) = get_tables();

    for byte in data.iter_mut() {
        *byte = inverse[*byte as usize];
    }
}

fn encode_permute(data: &mut [u8]) {
    let (forward, _) = get_tables();

    for byte in data.iter_mut() {
        *byte = forward[*byte as usize];
    }
}

fn read_bt_entry(file: &mut File, bref: Bref, bbt_map: &mut HashMap<u64, BlockInfo>, nbt_map: &mut HashMap<u32, Node>) -> Result<()> {
    //intermediate page
    let page = get_page(&file, bref.ib)?;

    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/f4ccb38a-930a-4db4-98df-a69c195926ba
    let page_trailer = &page[496..512]; //last 16 bytes
    let ptype = &page_trailer[0];
    // let w_sig = u16::from_le_bytes(page_trailer[2..4].try_into().unwrap());
    // let bid = bid_from_u64(u64::from_le_bytes(page_trailer[8..16].try_into().unwrap()));

    // println!("{:02X}", ptype);
    // println!("{}, {}, {}", ptype, w_sig, bid);
    // println!("{:#?}", page_trailer);
    // println!("{}", vec_u8_as_hex(page_trailer, true, " "));

/* ptypes
Value	Friendly name	Meaning	                    wSig value
0x80	ptypeBBT	    Block BTree page.	        Block or page signature (section 5.5).
0x81	ptypeNBT	    Node BTree page.	        Block or page signature (section 5.5).
0x82	ptypeFMap	    Free Map page.	            0x0000
0x83	ptypePMap	    Allocation Page Map page.	0x0000
0x84	ptypeAMap	    Allocation Map page.	    0x0000
0x85	ptypeFPMap	    Free Page Map page.	        0x0000
0x86	ptypeDL     	Density List page.	        Block or page signature (section 5.5).
*/

    if ptype==&0x80 { // Block BTree page.
        // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/4f0cd8e7-c2d0-4975-90a4-d417cfca77f8
        let rgentries = &page[0..488];
        let c_ent = &page[488];
        // let c_ent_max = &page[489];
        let cb_ent = &page[490];
        let c_level = &page[491];
        // println!("{}, {}, {}, {}", c_ent, c_ent_max, cb_ent, c_level);
        for ientry in 0..*c_ent {
            let starting_offset = ientry as usize * *cb_ent as usize;
            if *c_level==0 {
                //leaf page
                // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/53a4b926-8ac4-45c9-9c6d-8358d951dbcd
                let brefa = get_bref(rgentries[starting_offset..starting_offset+16].try_into().unwrap());
                let cb = u16::from_le_bytes(rgentries[starting_offset+16..starting_offset+18].try_into().unwrap());
                // let c_ref = u16::from_le_bytes(rgentries[starting_offset+18..starting_offset+20].try_into().unwrap());
                // println!("{:#?}: data len {}", brefa, cb);
                bbt_map.insert(brefa.bid, BlockInfo { offset: brefa.ib, size: cb as usize });
            } else {
                //intermediate page
                // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/bc8052a3-f300-4022-be31-f0f408fffca0
                // let btkey = u64::from_le_bytes(rgentries[starting_offset..starting_offset+8].try_into().unwrap());
                let brefa = get_bref(rgentries[starting_offset+8..starting_offset+24].try_into().unwrap());
                // println!("{}: {:#?}", btkey, bref);
                read_bt_entry(file, brefa, bbt_map, nbt_map)?;
            }
        }
    } else if ptype==&0x81 { //Node BTree page.
        let rgentries = &page[0..488];
        let c_ent = &page[488];
        // let c_ent_max = &page[489];
        let cb_ent = &page[490];
        let c_level = &page[491];
        // println!("{}, {}, {}, {}", c_ent, c_ent_max, cb_ent, c_level);
        for ientry in 0..*c_ent {
            let starting_offset = ientry as usize * *cb_ent as usize;
            if *c_level==0 {
                //leaf page
                // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/28fb2116-0998-4485-9844-9711b95603ba
                let nid = u64::from_le_bytes(rgentries[starting_offset+0..starting_offset+8].try_into().unwrap()) as u32;
                let bid_data = bid_from_u64(u64::from_le_bytes(rgentries[starting_offset+8..starting_offset+16].try_into().unwrap()));
                let bid_sub = bid_from_u64(u64::from_le_bytes(rgentries[starting_offset+16..starting_offset+24].try_into().unwrap()));
                let nid_parent = u32::from_le_bytes(rgentries[starting_offset+24..starting_offset+28].try_into().unwrap());
                //first 5 bytes are the nid type: https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/18d7644e-cb33-4e11-95c0-34d8a84fbff6
                let nid_type = (nid & 0x1F) as u8;
                // println!("{}, {}, {}, {}", nid, bid_data, bid_sub, nid_parent);

                let sub_nodes = get_sub_nodes(file, bbt_map, nid, bid_sub)?;
                let node: Node = Node { nid_type, data_bid: bid_data, sub_bid: bid_sub, parent: nid_parent, sub_nodes };
                // println!("node: {:?}", node);

                nbt_map.insert(nid, node);
            } else {
                //intermediate page
                // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/bc8052a3-f300-4022-be31-f0f408fffca0
                // let btkey = u64::from_le_bytes(rgentries[starting_offset..starting_offset+8].try_into().unwrap());
                let brefa = get_bref(rgentries[starting_offset+8..starting_offset+24].try_into().unwrap());
                // println!("{}: {:#?}", btkey, bref);
                read_bt_entry(file, brefa, bbt_map, nbt_map)?;
            }
        }
    } else {
        bail!("ptype {:02X} not configured", ptype)
    }

    Ok(())
}

fn get_sub_nodes(file: &mut File, bbt_map: &mut HashMap<u64, BlockInfo>, nid_parent: u32, bid_sub: u64) -> Result<HashMap<u32, Node>> {
    let mut sub_nodes:HashMap<u32, Node> = HashMap::new();
    if bid_sub==0 {
        return Ok(sub_nodes);
    }

    let block_info = bbt_map.get(&bid_sub).expect("There should always be a bbt entry");
    let block_data = get_block_data(file, &block_info, false)?;
    // decode_permute(&mut block_data);
    // println!("{}", string_from_utf16_as_vec_u8(&block_data));
    // println!("len: {}\n{}", block_data.len(), vec_u8_as_hex(&block_data, true, " "));
    //SLBLOCK
    let b_type = block_data[0];
    if b_type != 2 {
        bail!("btype (1 byte): Block type; MUST be set to 0x02."); //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/5182eb24-4b0b-4816-aa3f-719cc6e6b018
    }
    let c_level = block_data[1];
    let c_ent = u16::from_le_bytes([block_data[2], block_data[3]]); //The number of entries in the block.
    // println!("sub_bid: {}, {}, {}", b_type, c_level, c_ent);
    if c_level==0 {
        //SLBLOCK https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/5182eb24-4b0b-4816-aa3f-719cc6e6b018
        //rgentries Array of SLENTRY structures. The size is equal to the number of entries indicated by cEnt multiplied by the size of an SLENTRY (24 bytes)
        for i in 0..c_ent as usize {
            let sl_entry = &block_data[8+i*24..8+i*24+24];
            // println!("sl_entry: {}", vec_u8_as_hex(&sl_entry, true, " "));
            let sub_nid = u64::from_le_bytes(sl_entry[0..8].try_into().unwrap()) as u32;
            let sub_bid_data = bid_from_u64(u64::from_le_bytes(sl_entry[8..16].try_into().unwrap()));
            let sub_bid_sub = bid_from_u64(u64::from_le_bytes(sl_entry[16..24].try_into().unwrap()));
            let sub_nid_parent = &nid_parent;
            let sub_nid_type = (sub_nid & 0x1F) as u8;
            let sub_sub_nodes = get_sub_nodes(file, bbt_map, sub_nid, sub_bid_sub)?;
            let sub_node:Node = Node { nid_type: sub_nid_type, data_bid: sub_bid_data, sub_bid: sub_bid_sub, parent: *sub_nid_parent, sub_nodes: sub_sub_nodes };
            // println!("sub_node: {:?}", sub_node);
            sub_nodes.insert(sub_nid, sub_node);
        }
    } else if c_level==1 {
        //SIBLOCK https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/729fb9bd-060a-4bbc-9b3b-8f014b487dad
        bail!("TODO c_level 1 (SIBLOCK)")
    } else {
        bail!("c_level must b 0 (SLBLOCK) or 1 (SIBLOCK)")
    }

    Ok(sub_nodes)
}

fn read_bbt_entry(file: &File, bref: Bref) -> Result<()> {
    //leaf bbt entry

    Ok(())
}

fn check_magic_bytes(mut file: &File) -> bool {
    match file.seek(SeekFrom::Start(0)) {
        Ok(_) => {
            let mut buffer = vec![0u8; 4];
            match file.read_exact(&mut buffer) {
                Ok(_) => buffer == vec![33u8, 66, 68, 78],
                Err(_) => false
            }
        },
        Err(_) => false
    }
}

fn get_byte(mut file: &File, offset:u64) -> Result<[u8;1]> {
    file.seek(SeekFrom::Start(offset))?;
    let mut buffer = vec![0u8; 1];
    let _ = file.read_exact(&mut buffer);
    Ok(buffer.try_into().unwrap())
}

fn get_word(mut file: &File, offset:u64) -> Result<[u8;2]> {
    file.seek(SeekFrom::Start(offset))?;
    let mut buffer = vec![0u8; 2];
    let _ = file.read_exact(&mut buffer);
    Ok(buffer.try_into().unwrap())
}

// fn get_dword(mut file: &File, offset:u64) -> Result<[u8;4]> {
//     file.seek(SeekFrom::Start(offset))?;
//     let mut buffer = vec![0u8; 4];
//     let _ = file.read_exact(&mut buffer);
//     Ok(buffer.try_into().unwrap())
// }

fn get_qword(mut file: &File, offset:u64) -> Result<[u8;8]> {
    file.seek(SeekFrom::Start(offset))?;
    let mut buffer = vec![0u8; 8];
    let _ = file.read_exact(&mut buffer);
    Ok(buffer.try_into().unwrap())
}

// fn get_oword(mut file: &File, offset:u64) -> Result<[u8;16]> {
//     file.seek(SeekFrom::Start(offset))?;
//     let mut buffer = vec![0u8; 16];
//     let _ = file.read_exact(&mut buffer);
//     Ok(buffer.try_into().unwrap())
// }

fn bid_from_u64(input:u64) -> u64 {
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/d3155aa1-ccdd-4dee-a0a9-5363ccca5352
    // first two bits should be ignored
    // Mask with all bits = 1 except the top 2 bits
    input & !(0b11 << 62)
}

fn get_bid(file: &File, offset:u64) -> Result<u64> {
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/d3155aa1-ccdd-4dee-a0a9-5363ccca5352
    // first two bits should be ignored
    Ok(bid_from_u64(get_u64(file, offset)?))
}

fn get_page(mut file: &File, offset:u64) -> Result<[u8;512]> {
    file.seek(SeekFrom::Start(offset))?;
    let mut buffer = vec![0u8; 512];
    let _ = file.read_exact(&mut buffer);
    Ok(buffer.try_into().unwrap())
}

fn get_bref(bytes:[u8; 16]) -> Bref {
    Bref {
        bid: bid_from_u64(u64::from_le_bytes(bytes[..8].try_into().unwrap())),
        ib: u64::from_le_bytes(bytes[8..].try_into().unwrap())
    }
}

fn get_u8(file: &File, offset:u64) -> Result<u8> { Ok(get_byte(file, offset)?[0]) }
fn get_u16(file: &File, offset:u64) -> Result<u16> { Ok(u16::from_le_bytes(get_word(file, offset)?)) }
// fn get_u32(file: &File, offset:u64) -> Result<u32> { Ok(u32::from_le_bytes(get_dword(file, offset)?)) }
fn get_u64(file: &File, offset:u64) -> Result<u64> { Ok(u64::from_le_bytes(get_qword(file, offset)?)) }


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_hid_a() {
        let hid_bytes = vec![0x40, 0x00, 0x00, 0x00];
        //LE byte order = 00 00 00 40
        // bits = 0000 0000 0000 0000 : 0000 0000 010 : 0 0000
        let hid_value: u32 = u32::from_le_bytes(hid_bytes.try_into().unwrap());
        let hid = Hid::new(hid_value);
        let result = Hid { hid: hid_value, hid_type: 0, hid_index: 2, hid_block_index: 0};
        assert_eq!(hid, result);
    }

    #[test]
    fn test_new_hid_b() {
        let hid_bytes = vec![0xE0, 0x00, 0x01, 0x00];
        //LE byte order, 2byte chunks = 00 01 00 E0
        // bits = 0000 0000 0000 0001 : 0000 0000 111 : 0 0000
        let hid_value: u32 = u32::from_le_bytes(hid_bytes.try_into().unwrap());
        let hid = Hid::new(hid_value);
        let result = Hid { hid: hid_value, hid_type: 0, hid_index: 7, hid_block_index: 1};
        assert_eq!(hid, result);
    }
}
