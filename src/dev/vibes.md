I am writing a rust program to read a unicode pst file at byte level.

I have read a heap on node and get the properties based on the BTH Header and the Page Map.
I can tracerse the properties and read the propType and all is well for most.
I am stuck at this bit:
Reference: 2.3.3.2 HNID https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/7ac490ce-31af-4a75-97df-eb9d07a003fd
"An HNID is a 32-bit hybrid value that represents either an HID or an NID. The determination is made by examining the hidType (or equivalently, nidType) value. The HNID refers to an HID if the hidType is NID_TYPE_HID. Otherwise, the HNID refers to an NID.
An HNID that refers to an HID indicates that the item is stored in the data block. An HNID that refers to an NID indicates that the item is stored in the subnode block, and the NID is the local NID under the subnode where the raw data is located."
So I have an item (property 007D: TransportMessageHeaders) where hidType is not NID_TYPE_HID, and so I need to find the subnode block based on the u32 nid.

I think I need to get a subnode, but am not sure how. They do not seem to be part of the node hashmap I made from reading the BT entries.

