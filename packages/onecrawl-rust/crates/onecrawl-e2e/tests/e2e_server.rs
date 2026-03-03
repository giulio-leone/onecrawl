//! E2E tests for the OneCrawl HTTP server.
//! Starts the server on a free port and hits endpoints with reqwest.

use std::net::TcpListener;
use std::time::Duration;

fn get_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

async fn wait_for_server(port: u16) {
    for _ in 0..50 {
        if reqwest::get(format!("http://127.0.0.1:{}/health", port))
            .await
            .is_ok()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("Server failed to start on port {}", port);
}

#[tokio::test]
async fn server_health_endpoint() {
    let port = get_free_port();
    let _server = tokio::spawn(async move {
        let _ = onecrawl_server::serve::start_server(port).await;
    });
    wait_for_server(port).await;

    let resp = reqwest::get(format!("http://127.0.0.1:{}/health", port))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "onecrawl-server");
}

#[tokio::test]
async fn server_instances_empty() {
    let port = get_free_port();
    let _server = tokio::spawn(async move {
        let _ = onecrawl_server::serve::start_server(port).await;
    });
    wait_for_server(port).await;

    let resp = reqwest::get(format!("http://127.0.0.1:{}/instances", port))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["instances"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn server_profiles_empty() {
    let port = get_free_port();
    let _server = tokio::spawn(async move {
        let _ = onecrawl_server::serve::start_server(port).await;
    });
    wait_for_server(port).await;

    let resp = reqwest::get(format!("http://127.0.0.1:{}/profiles", port))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["profiles"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn server_404_on_unknown_route() {
    let port = get_free_port();
    let _server = tokio::spawn(async move {
        let _ = onecrawl_server::serve::start_server(port).await;
    });
    wait_for_server(port).await;

    let resp = reqwest::get(format!("http://127.0.0.1:{}/nonexistent", port))
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn server_create_profile() {
    let port = get_free_port();
    let _server = tokio::spawn(async move {
        let _ = onecrawl_server::serve::start_server(port).await;
    });
    wait_for_server(port).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://127.0.0.1:{}/profiles", port))
        .json(&serde_json::json!({"name": "test-profile"}))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["profile"]["name"].as_str().unwrap().contains("test-profile"));
}

#[tokio::test]
async fn server_concurrent_health_checks() {
    let port = get_free_port();
    let _server = tokio::spawn(async move {
        let _ = onecrawl_server::serve::start_server(port).await;
    });
    wait_for_server(port).await;

    let mut handles = vec![];
    for _ in 0..10 {
        let url = format!("http://127.0.0.1:{}/health", port);
        handles.push(tokio::spawn(async move {
            reqwest::get(&url).await.unwrap().status()
        }));
    }

    for h in handles {
        assert_eq!(h.await.unwrap(), 200);
    }
}
