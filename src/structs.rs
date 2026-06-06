use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::{enums::{NidType, PropId, PropType, RecipientType}};

#[derive(Debug)]
#[allow(dead_code)]
pub struct Bref {
    pub bid: u64,
    pub ib: u64,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct BlockInfo {
    pub offset: u64,
    pub size: usize,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Node {
    pub nid_type: NidType,
    pub data_bid: u64,
    pub sub_bid: u64,
    pub parent: u32,
    pub sub_nodes:HashMap<u32, Node>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct HNBlock {
    pub index: usize, //This index gives the type of block. First is header for next 8, then HNBITMAPHDR every 128 blocks (see https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/822e2327-b29d-4ec4-91be-45637a438d40)
    //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/8e4ae05c-3c24-4103-b7e5-ffef6f244834
    //HNHDR
    // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/8e4ae05c-3c24-4103-b7e5-ffef6f244834
    pub ib_hnpm: u16,
    pub b_sig: u8,
    pub b_client_sig: u8,
    pub hid_user_root: Hid,
    pub rgb_fill_level: u32,
    //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/291653c0-b347-4c5b-ba41-85ad780b4ba4
    pub c_alloc: u16,
    pub c_free: u16,
    //https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/2dd1a95a-c8b1-4ac5-87d1-10cb8de64053
    pub bth_chunks: Vec<Vec<u8>>,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub struct Hid {
    pub hid: u32,
    pub hid_type: u8,
    pub hid_index: u16,
    pub hid_block_index: u16,
}
impl Hid {
    pub fn new(i: u32) -> Self {
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

#[derive(Debug)]
#[allow(dead_code)]
pub struct PropertyEntry {
    pub property: Property,
    pub prop_value: [u8;4],
    pub value_string: String,
    pub value_bool: Option<bool>,
    pub value_i16: Option<i16>,
    pub value_i32: Option<i32>,
    pub value_i64: Option<i64>,
    pub value_f32: Option<f32>,
    pub value_f64: Option<f64>,
    pub value_datetime: Option<DateTime<Utc>>,
    pub value_binary: Option<Vec<u8>>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ColumnEntry {
    pub tag: u32,
    pub data_offset: u16,
    pub data_size: u8,
    pub ceb_bit: u8,
    pub property: Property,
    pub value_string: Option<Vec<String>>,
    pub value_bool: Option<Vec<bool>>,
    pub value_i16: Option<Vec<i16>>,
    pub value_i32: Option<Vec<i32>>,
    pub value_i64: Option<Vec<i64>>,
    pub value_f32: Option<Vec<f32>>,
    pub value_f64: Option<Vec<f64>>,
    pub value_datetime: Option<Vec<DateTime<Utc>>>,
    pub value_binary: Option<Vec<Vec<u8>>>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct TableRows {
    pub num_rows: usize,
    pub rows: HashMap<PropId, ColumnEntry>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Property {
    pub prop_id: PropId,
    pub prop_type: PropType,
}
impl Property {
    pub fn new(prop_tag: [u8;4]) -> Self {
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
pub struct Recipient {
    pub display_name: String,
    pub email_address: String,
    pub recipient_type: RecipientType,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Message {
	pub received_time: DateTime<Utc>,
	pub subject: String,
	pub conversation: String,
	pub sender_name: String,
	pub sender_email_address: String,
	pub sent_time: Option<DateTime<Utc>>,
	pub text: String,
	pub html: String,
	pub recipients: Vec<Recipient>,
	pub sub_msgs: Vec<Message>,
	// pub attachments: Vec<MsgAttachment>,
}


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
