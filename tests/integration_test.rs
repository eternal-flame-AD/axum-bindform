use axum::http::StatusCode;
use axum_bindform::{BindForm, TryBindForm};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Human {
    name: String,
    age: u8,
}

async fn greet_human(BindForm(form): BindForm<Human>) -> String {
    format!("Hello {} year old named {}!", form.age, form.name)
}

async fn try_greet_human(
    TryBindForm(form): TryBindForm<Human>,
) -> Result<String, (StatusCode, String)> {
    let form = form.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Error parsing form: {}", e),
        )
    })?;
    Ok(format!("Hello {} year old named {}!", form.age, form.name))
}

#[tokio::test]
async fn test_bind_forms() {
    let app = axum::Router::new()
        .route("/greet", axum::routing::get(greet_human).post(greet_human))
        .route(
            "/try_greet",
            axum::routing::get(try_greet_human).post(try_greet_human),
        )
        .into_make_service();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        axum::Server::bind(&"127.0.0.1:3000".parse().unwrap())
            .serve(app)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            })
            .await
            .unwrap();
    });

    let client = reqwest::Client::new();

    assert_eq!(
        client
            .post("http://localhost:3000/greet")
            .json(&Human {
                name: "John".to_string(),
                age: 32,
            })
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "Hello 32 year old named John!"
    );

    let xml_str = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Human>
            <name>John</name>
            <age>32</age>
        </Human>"#;

    assert_eq!(
        client
            .post("http://localhost:3000/greet")
            .header("content-type", "application/xml")
            .body(xml_str)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "Hello 32 year old named John!"
    );

    assert_eq!(
        client
            .get("http://localhost:3000/greet?name=John&age=32")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "Hello 32 year old named John!"
    );

    let error_response = client
        .get("http://localhost:3000/try_greet?name=John")
        .send()
        .await
        .unwrap();

    assert!(error_response.status().is_client_error());
    assert!(error_response
        .text()
        .await
        .unwrap()
        .contains("missing field `age`"));

    drop(shutdown_tx);
}
