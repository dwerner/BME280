extern crate BME280;

#[test]
fn it_can_initialize() {
    let result = BME280::create(0x77, 0);
    match result {
        Ok(device) => assert!(true),
        Err(_) => assert!(false)
    }
}