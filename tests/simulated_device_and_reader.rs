use anyhow::Result;

use common::*;

mod common;

#[test]
fn simulated_device_and_reader_interaction() -> Result<()> {
    // Device initialization and engagement
    let (engaged_state, qr_code_uri) = Device::initialise_session()?;

    // Reader processing QR and requesting the necessary fields
    let (mut reader_session_manager, request) = Device::establish_session(qr_code_uri)?;

    // Device accepting request
    let (device_session_manager, requested_items) = Device::handle_request(engaged_state, request)?;

    // Prepare response with required elements
    let response = Device::create_response(
        device_session_manager,
        requested_items,
        &Device::create_signing_key()?,
    )?;

    // Reader Processing mDL data
    Reader::handle_device_response(&mut reader_session_manager, response)?;

    Ok(())
}
