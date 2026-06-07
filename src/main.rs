// C:\Users\hrag\Sync\Programming\cfbf_cdf_ole_format_filetype_notes.md
// 

use std::path::Path;

use anyhow::{Result};
use log::*;

use pst_reader::{enums::*, structs::PstFile, *};

// mod consts;

mod enums;
// use enums::*;

// mod structs;

// mod supporting_functions;
// use supporting_functions::*;

fn main() -> Result<()> {
    helper_lib::setup_logger(LevelFilter::Debug, None, "", "html5ever");
    
    info!("start");

    println!("Hello, world!");

    let pst_path = Path::new("./dev/test.pst");

    let pst_file = PstFile::new(pst_path)?;

    for (_, node) in &pst_file.nbt_map {
        // println!("{}: {:?}", nid, node);
        // println!("{:02X}", node.nid_type);
        if node.data_bid > 0 {
            // let block_info = pst_file.bbt_map.get(&node.data_bid).expect("There should always be a bbt entry");
            // let mut block_data = get_block_data(&mut file, &block_info, false)?;
            // println!("{}: {:?}", nid, node);
            // https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/18d7644e-cb33-4e11-95c0-34d8a84fbff6
            if node.nid_type==NidType::NormalFolder { //NID_TYPE_NORMAL_FOLDER
                // let property_entries = get_properties(None, &mut block_data, &node, &b_crypt_method, &mut file, &bbt_map)?;
                // // println!("{:#?}", property_entries);
                // for propery_entry in property_entries {
                //     println!("  {:?} ({:?}): {}", propery_entry.prop_id, propery_entry.prop_type, propery_entry.value_string)
                // }
            } else if node.nid_type==NidType::NormalMessage { // NID_TYPE_NORMAL_MESSAGE
                let msg = &pst_file.get_message(node)?;
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

                // let prop_ids: Option<Vec<PropId>> = Some(vec![PropId::Subject]);
                // let property_entries = get_object_properties(&prop_ids, &mut block_data, &node, &b_crypt_method, &mut file, &bbt_map)?;
                // // println!("property_entries: {:#?}", property_entries);
                // // for propery_entry in property_entries {
                // //     println!("  {:?}: {}", propery_entry.property, propery_entry.value_string);
                // // }
                // // println!("{:?}", property_entries[&PropId::Subject]);
                // let subject: String = get_prop_string(&property_entries, &PropId::Subject);
                // // println!();
                // println!("{}", subject);
                // println!("{}, {}", subject.len(), "FW: Daily Personnel Costs NZ".len());
                // println!("{:?}", subject.as_bytes());
                // println!("{}", subject=="FW: Daily Personnel Costs NZ".to_string());

                // let msg = get_message(node, &mut file, &bbt_map, &b_crypt_method)?;
                // println!("{:#?}", msg);

                // let recipients = get_recipients(node, &mut file, &bbt_map, &b_crypt_method)?;
                // println!("{:#?}", recipients);
                // break

                // if subject=="FW: Daily Personnel Costs NZ".to_string() {
                //     // let attachments = get_file_attachments(node, &mut file, &bbt_map, &b_crypt_method)?;
                //     // println!("{:#?}", attachments);
                //     let sub_msgs = get_message_attachments(node, &mut file, &bbt_map, &b_crypt_method)?;
                //     println!("{:#?}", sub_msgs);

                //     break
                // }
                // if subject=="RE: Daily Personnel Costs NZ".to_string() {
                //     let attachments = get_file_attachments(node, &mut file, &bbt_map, &b_crypt_method)?;
                //     // println!("{:#?}", attachments);
                //     for att in attachments {
                //         println!("  {}", att.filename);
                //         let mut path = PathBuf::from("/home/ray/MEGA/Rays/temp");
                //         path.push(att.filename);
                //         let _ = std::fs::write(path, att.bytes)?;
                //     }

                //     break
                // }
                // break
            }
        }
    }

    info!("end");

    Ok(())
}

