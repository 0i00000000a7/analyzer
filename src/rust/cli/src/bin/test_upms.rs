use bms_core::upms::upms_to_bms;
fn main() {
    let m = vec![
        vec![0,0,0],
        vec![1,1,1],
        vec![2,1,0],
        vec![1,1,1],
    ];
    match upms_to_bms(&m) {
        Ok(r) => println!("OK: {:?}", r),
        Err(e) => println!("ERR: {}", e),
    }
}
