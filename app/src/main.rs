use serialize_macro::{SerializeNumberStruct, DeserializeNumberStruct};
use serialize_macro_traits::{Serialize, Deserialize,Error};

#[derive(SerializeNumberStruct, DeserializeNumberStruct,Debug)]
struct Swap {
    qty_1: u64,
    qty_2: String,
    qty_3: i32
}


fn main() {
    println!("Hello, world!");
    let s = Swap {
        qty_1: 1,
        qty_2: "Hello dsfg".to_string(),
        qty_3: 1000
    };
    let bytes = s.serialize();
    println!("{:?}", bytes);
    let s2 = Swap::deserialize(&bytes).unwrap();
    println!("{:?}", s2);
}
