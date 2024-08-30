use crate::suite::TestApp;
use integrationos_domain::{prefix::IdPrefix, Id};
use mark_flaky_tests::flaky;
use oauth_refresh::{Token, TokenExt};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use uuid::Uuid;

#[actix::test]
async fn returns_401_for_missing_headers() {
    // Arrange
    let application = TestApp::spawn(HashMap::new()).await;
    // Act
    let path = format!("integration/trigger/{}", Uuid::nil());
    let response = application.post(path, "", None).await;
    // Assert
    assert_eq!(401, response.status().as_u16());
}

#[actix::test]
async fn returns_404_for_invalid_prefix_id() {
    // Arrange
    let application = TestApp::spawn(HashMap::new()).await;
    let event_access = application.insert_event_access().await;
    let event_access_token = event_access.access_key;
    let token = Token
        .generate(application.configuration().clone(), 1)
        .expect("Failed to generate token");

    let headers = HeaderMap::from_iter(vec![
        (
            HeaderName::from_static("x-integrationos-secret"),
            HeaderValue::from_str(&event_access_token).expect("Failed to create header value"),
        ),
        (
            HeaderName::from_static("x-integrationos-admin-token"),
            HeaderValue::from_str(&format!("Bearer {}", token))
                .expect("Failed to create header value"),
        ),
    ]);
    let path = format!("integration/trigger/{}", Uuid::nil());
    // Act
    let response = application.post(path, "", Some(headers)).await;
    // Assert
    assert_eq!(
        "text/plain; charset=utf-8",
        response
            .headers()
            .get("content-type")
            .expect("Failed to get content type")
            .to_str()
            .expect("Failed to convert content type to string")
    );
    assert_eq!(
        "Argument provided is invalid: Invalid ID prefix: 00000000-0000-0000-0000-000000000000",
        response.text().await.expect("Failed to get response text")
    );
}

#[actix::test]
async fn returns_401_for_non_existent_event_access() {
    // Arrange
    let application = TestApp::spawn(HashMap::new()).await;
    let event_access = "sk_live_1_Gom7umYOtRPyCbx4o2XNIlM32-2wf2dPI6s7nsdlWeXuhRj1rgDEvFeYAVckQvwG-5IUzRHGWnloNx2fci7IdFcdlTqYAuUuj6QQZPOvS2sxGK4YKnkmS1UFqcXFDCsSYZxASBaqJaBZA1HMEVuv61-cepuCBJccX90hXqQlKZvZ5s0i8hRZszeCA9b3H18paLy7";
    let token = Token
        .generate(application.configuration().clone(), 1)
        .expect("Failed to generate token");
    let headers = HeaderMap::from_iter(vec![
        (
            HeaderName::from_static("x-integrationos-secret"),
            HeaderValue::from_str(event_access).expect("Failed to create header value"),
        ),
        (
            HeaderName::from_static("x-integrationos-admin-token"),
            HeaderValue::from_str(&format!("Bearer {}", token))
                .expect("Failed to create header value"),
        ),
    ]);
    let id = Id::now(IdPrefix::ConnectionModelDefinition);
    let path = format!("integration/trigger/{}", id);
    // Act
    let response = application.post(path, "", Some(headers)).await;
    // Assert
    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        "text/plain; charset=utf-8",
        response
            .headers()
            .get("content-type")
            .expect("Failed to get content type")
            .to_str()
            .expect("Failed to convert content type to string")
    );
    assert_eq!(
        format!("No event access found for key: {}", event_access),
        response.text().await.expect("Failed to get response text")
    );
}

#[actix::test]
#[flaky]
async fn returns_404_inexistent_event() {
    // Arrange
    let application = TestApp::spawn(HashMap::new()).await;
    let event_access = application.insert_event_access().await;
    let event_access_token = event_access.access_key;

    let token = Token
        .generate(application.configuration().clone(), 1)
        .expect("Failed to generate token");

    let headers = HeaderMap::from_iter(vec![
        (
            HeaderName::from_static("x-integrationos-secret"),
            HeaderValue::from_str(&event_access_token).expect("Failed to create header value"),
        ),
        (
            HeaderName::from_static("x-integrationos-admin-token"),
            HeaderValue::from_str(&format!("Bearer {}", token))
                .expect("Failed to create header value"),
        ),
    ]);

    let id = Id::now(IdPrefix::ConnectionModelDefinition);
    let path = format!("integration/trigger/{}", id);
    // Act
    let response = application.post(path, "", Some(headers)).await;
    // Assert
    let msg = format!(
        "{{\"passthrough\":{{\"type\":\"NotFound\",\"code\":2005,\"status\":404,\"key\":\"err::application::not_found\",\"message\":\"Connection with id {} not found\"}}}}",
        id
    );
    assert_eq!(404, response.status().as_u16());
    assert_eq!(
        "application/json",
        response
            .headers()
            .get("content-type")
            .expect("Failed to get content type")
            .to_str()
            .expect("Failed to convert content type to string")
    );
    assert_eq!(
        msg,
        response.text().await.expect("Failed to get response text")
    );
}
