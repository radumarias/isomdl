include!("../examples/simulated_device_and_reader.rs");

#[test]
fn test_device_and_reader_interaction() {
    run_simulated_device_and_reader_interaction().unwrap();
}
