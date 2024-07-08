#![doc(test(attr(deny(warnings))))]
//! ISO/IEC DIS 18013-5 mDL implementation in Rust.
//!
//! # Examples
//!
//! This example demonstrates a simulated device and reader interaction.  
//! The reader requests the `age_over_21` element, and the device responds with that value.
//! The flow is as follows:
//!
//! 1. Device initialization and engagement:
//!     - The device creates a QR code containing `DeviceEngagement` data, which includes its public key.
//!     - Internally:
//!         - The device initializes with the mDL data, private key, and public key.
//! 2. Reader processing QR and requesting needed fields:
//!     - The reader processes the QR code and creates a request for the `age_over_21` element.
//!     - Internally:
//!         - Generates its private and public keys.
//!         - Initiates a key exchange, and generates the session keys.
//!         - The request is encrypted with the reader's session key.
//! 3. Device accepting request and responding:
//!     - The device receives the request and creates a response with the `age_over_21` element.
//!     - Internally:
//!         - Initiates the key exchange, and generates the session keys.
//!         - Decrypts the request with the reader's session key.
//!         - Parse and validate it creating error response if needed.
//!         - The response is encrypted with the device's session key.
//! 4. Reader Processing mDL data:
//!     - The reader processes the response and prints the value of the `age_over_21` element.
//! <!-- INCLUDE-RUST: examples/simulated_device_and_reader.rs -->
//! ```
//! use std::sync::{Arc, Mutex};
//!
//! use anyhow::{Context, Result};
//! use signature::{SignatureEncoding, Signer};
//! use uuid::Uuid;
//!
//! use isomdl::definitions::device_engagement::{CentralClientMode, DeviceRetrievalMethods};
//! use isomdl::definitions::device_request::{DataElements, DocType, Namespaces};
//! use isomdl::definitions::helpers::NonEmptyMap;
//! use isomdl::definitions::{self, BleOptions, DeviceRetrievalMethod};
//! use isomdl::presentation::device::{Document, Documents, RequestedItems};
//! use isomdl::presentation::{device, reader, Stringify};
//!
//! const DOC_TYPE: &str = "org.iso.18013.5.1.mDL";
//! const NAMESPACE: &str = "org.iso.18013.5.1";
//! const AGE_OVER_21_ELEMENT: &str = "age_over_21";
//!
//! struct SessionData {
//!     state: Arc<SessionManagerEngaged>,
//!     qr_code_uri: String,
//! }
//!
//! struct RequestData {
//!     session_manager: Arc<SessionManager>,
//! }
//!
//! struct SessionManager {
//!     inner: Mutex<device::SessionManager>,
//!     items_requests: RequestedItems,
//!     key: Arc<p256::ecdsa::SigningKey>,
//! }
//!
//! struct SessionManagerEngaged(device::SessionManagerEngaged);
//!
//! fn main() -> Result<()> {
//!     run_simulated_device_and_reader_interaction()
//! }
//!
//! pub fn run_simulated_device_and_reader_interaction() -> Result<()> {
//!     let mdl_encoded = include_str!("../examples_data/stringified-mdl.txt");
//!     let key: Arc<p256::ecdsa::SigningKey> = Arc::new(
//!         p256::SecretKey::from_sec1_pem(include_str!("../examples_data/sec1.pem"))
//!             .inspect_err(|e| println!("Error: {:?}", e))
//!             .unwrap()
//!             .into(),
//!     );
//!
//!     // Parse the mDL
//!     let docs = parse_mdl(mdl_encoded)?;
//!
//!     // Device initialization and engagement
//!     let session_data = initialise_session(docs, Uuid::new_v4())?;
//!
//!     // Reader processing QR and requesting needed fields
//!     let (mut reader_session_manager, request) = establish_reader_session(session_data.qr_code_uri)?;
//!
//!     // Device accepting request
//!     let request_data = handle_request(
//!         session_data.state,
//!         &mut reader_session_manager,
//!         request,
//!         key.clone(),
//!     )?;
//!     if request_data.is_none() {
//!         println!("Errors sent, terminating.");
//!         return Ok(());
//!     }
//!     let request_data = request_data.unwrap();
//!
//!     // Prepare response with required elements
//!     let response = create_response(request_data.session_manager.clone())?;
//!
//!     // Reader Processing mDL data
//!     reader_handle_device_response(&mut reader_session_manager, response)?;
//!
//!     Ok(())
//! }
//!
//! /// Check if there were any errors and sign them if needed, returning the response error.
//! fn get_errors(session_manager: Arc<SessionManager>) -> Result<Option<Vec<u8>>> {
//!     sign_pending_and_retrieve_response(session_manager, None)
//! }
//!
//! /// Parse the mDL encoded string into a [Documents] object.
//! fn parse_mdl(encoded: &str) -> Result<NonEmptyMap<DocType, Document>> {
//!     let mdl = Document::parse(encoded.to_string()).context("could not parse mDL")?;
//!     let docs = Documents::new(DOC_TYPE.to_string(), mdl);
//!     Ok(docs)
//! }
//!
//! /// Creates a QR code containing `DeviceEngagement` data, which includes its public key.
//! fn initialise_session(docs: Documents, uuid: Uuid) -> Result<SessionData> {
//!     let drms = DeviceRetrievalMethods::new(DeviceRetrievalMethod::BLE(BleOptions {
//!         peripheral_server_mode: None,
//!         central_client_mode: Some(CentralClientMode { uuid }),
//!     }));
//!
//!     let session = device::SessionManagerInit::initialise(docs, Some(drms), None)
//!         .context("failed to initialize device")?;
//!     let mut ble_ident = session
//!         .ble_ident()
//!         .map(hex::encode)
//!         .context("could not encode hex BLE ident")?;
//!     ble_ident.insert_str(0, "0x");
//!
//!     let (engaged_state, qr_code_uri) = session
//!         .qr_engagement()
//!         .context("could not generate qr engagement")?;
//!     Ok(SessionData {
//!         state: Arc::new(SessionManagerEngaged(engaged_state)),
//!         qr_code_uri,
//!     })
//! }
//!
//! /// Establishes the reader session from the given QR code and create request for needed elements.
//! fn establish_reader_session(qr: String) -> Result<(reader::SessionManager, Vec<u8>)> {
//!     let requested_elements = Namespaces::new(
//!         NAMESPACE.into(),
//!         DataElements::new(AGE_OVER_21_ELEMENT.to_string(), false),
//!     );
//!     let (reader_sm, session_request, _ble_ident) =
//!         reader::SessionManager::establish_session(qr, requested_elements)
//!             .context("failed to establish reader session")?;
//!     Ok((reader_sm, session_request))
//! }
//!
//! /// The Device handles the request from the reader and creates the `RequestData` context.
//! fn handle_request(
//!     state: Arc<SessionManagerEngaged>,
//!     reader_session_manager: &mut reader::SessionManager,
//!     request: Vec<u8>,
//!     key: Arc<p256::ecdsa::SigningKey>,
//! ) -> Result<Option<RequestData>> {
//!     let (session_manager, items_requests) = {
//!         let session_establishment: definitions::SessionEstablishment =
//!             serde_cbor::from_slice(&request).context("could not deserialize request")?;
//!         state
//!             .0
//!             .clone()
//!             .process_session_establishment(session_establishment)
//!             .context("could not process process session establishment")?
//!     };
//!     let session_manager = Arc::new(SessionManager {
//!         inner: Mutex::new(session_manager),
//!         items_requests: items_requests.clone(),
//!         key,
//!     });
//!     // Propagate any errors back to the reader
//!     if let Ok(Some(response)) = get_errors(session_manager.clone()) {
//!         let res = reader_session_manager.handle_response(&response)?;
//!         println!("Reader: {res:?}");
//!         return Ok(None);
//!     };
//!
//!     Ok(Some(RequestData { session_manager }))
//! }
//!
//! // Prepare response with required elements.
//! fn create_response(session_manager: Arc<SessionManager>) -> Result<Vec<u8>> {
//!     let permitted_items = [(
//!         DOC_TYPE.to_string(),
//!         [(NAMESPACE.to_string(), vec![AGE_OVER_21_ELEMENT.to_string()])]
//!             .into_iter()
//!             .collect(),
//!     )]
//!     .into_iter()
//!     .collect();
//!     session_manager
//!         .inner
//!         .lock()
//!         .unwrap()
//!         .prepare_response(&session_manager.items_requests, permitted_items);
//!     sign_pending_and_retrieve_response(session_manager.clone(), Some(1))?
//!         .ok_or_else(|| anyhow::anyhow!("no response to return"))
//! }
//!
//! fn sign_pending_and_retrieve_response(
//!     session_manager: Arc<SessionManager>,
//!     expected_to_sign: Option<usize>,
//! ) -> Result<Option<Vec<u8>>> {
//!     let mut signed = 0;
//!     loop {
//!         let mut guard = session_manager.inner.lock().unwrap();
//!         if let Some((_, payload)) = guard.get_next_signature_payload() {
//!             if let Some(expected_to_sign) = expected_to_sign {
//!                 if signed >= expected_to_sign {
//!                     anyhow::bail!(
//!                         "expected to sign {} documents, but there are more",
//!                         expected_to_sign
//!                     );
//!                 }
//!             }
//!             let signature: p256::ecdsa::Signature = session_manager.key.sign(payload);
//!             guard
//!                 .submit_next_signature(signature.to_der().to_vec())
//!                 .context("failed to submit signature")?;
//!             signed += 1;
//!         } else {
//!             break;
//!         }
//!     }
//!     Ok(session_manager.inner.lock().unwrap().retrieve_response())
//! }
//!
//! /// Reader Processing mDL data.
//! fn reader_handle_device_response(
//!     reader_sm: &mut reader::SessionManager,
//!     response: Vec<u8>,
//! ) -> Result<()> {
//!     let res = reader_sm.handle_response(&response)?;
//!     println!("{:?}", res);
//!     Ok(())
//! }
//! ```
pub use cose_rs;

pub mod definitions;
pub mod issuance;
pub mod presentation;

pub mod macros {
    pub use isomdl_macros::{FromJson, ToCbor};
}
