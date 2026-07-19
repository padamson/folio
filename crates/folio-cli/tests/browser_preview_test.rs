//! Browser E2E tests for the preview dashboard.
//!
//! These drive a real Chromium via playwright-rs against the axum preview
//! server. The tests intentionally use `start_preview_server_with_state`
//! rather than `start_preview_with_browser`: the latter opens the user's
//! real default browser (a best-effort convenience shim), which a test
//! suite has no business doing. The playwright-controlled browser here
//! stands in as the "user's browser".

use folio_core::preview::PreviewState;
use playwright_rs::{expect, protocol::Playwright};

/// Build a 3-batch preview state with real thumbnails from the test fixture.
fn three_batch_state(names: [&str; 3]) -> PreviewState {
    use folio_core::preview::{generate_thumbnail, BatchPreview, WorkflowStage};
    use std::path::PathBuf;

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures/sample-with-exif.jpg");

    let thumbnail = generate_thumbnail(&fixture_path, 100).expect("Failed to generate thumbnail");

    let batches = vec![
        BatchPreview {
            index: 0,
            name: Some(names[0].to_string()),
            file_count: 51,
            first_photo: Some("DSC_0001.JPG".to_string()),
            last_photo: Some("DSC_0050.JPG".to_string()),
            first_thumbnail: Some(thumbnail.clone()),
            last_thumbnail: Some(thumbnail.clone()),
            time_range: Some("14:00 - 16:30".to_string()),
        },
        BatchPreview {
            index: 1,
            name: Some(names[1].to_string()),
            file_count: 32,
            first_photo: Some("DSC_0051.JPG".to_string()),
            last_photo: Some("DSC_0082.JPG".to_string()),
            first_thumbnail: Some(thumbnail.clone()),
            last_thumbnail: Some(thumbnail.clone()),
            time_range: Some("17:00 - 19:00".to_string()),
        },
        BatchPreview {
            index: 2,
            name: Some(names[2].to_string()),
            file_count: 15,
            first_photo: Some("DSC_0083.JPG".to_string()),
            last_photo: Some("DSC_0097.JPG".to_string()),
            first_thumbnail: Some(thumbnail.clone()),
            last_thumbnail: Some(thumbnail.clone()),
            time_range: Some("20:00 - 21:00".to_string()),
        },
    ];

    PreviewState {
        batch_count: 3,
        current_stage: WorkflowStage::Group,
        batches,
    }
}

/// Static render: batch cards, workflow progress, and thumbnails all display.
#[tokio::test]
async fn test_browser_preview_e2e_static_render() {
    let state = three_batch_state([
        "thanksgiving-arrival",
        "thanksgiving-dinner",
        "thanksgiving-cleanup",
    ]);

    let server = folio_core::preview::start_preview_server_with_state(state)
        .await
        .expect("Failed to start preview server");
    let preview_url = server.url();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");
    page.goto(&preview_url, None)
        .await
        .expect("Failed to navigate");

    // Micro-slice 1: page heading
    expect(page.locator("h1"))
        .to_have_text("Hello Preview")
        .await
        .unwrap();

    // Micro-slice 2: batch count
    expect(page.locator(".batch-count"))
        .to_have_text("3 batches")
        .await
        .unwrap();

    // Micro-slice 3: workflow progress. The element renders each stage on
    // its own line, and playwright-rs `to_contain_text` does not normalize
    // whitespace the way upstream Playwright does (reported upstream), so
    // compare against a whitespace-normalized string.
    let progress_text = page
        .locator(".workflow-progress")
        .text_content()
        .await
        .expect("Failed to get workflow progress text")
        .expect("Workflow progress should have text");
    let normalized = progress_text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        normalized.contains("Scan → Group → Name → Review → Copy"),
        "Workflow progress should show the stage sequence, got: {}",
        normalized
    );

    // Micro-slice 4: current stage highlighted
    expect(page.locator(".workflow-stage.active"))
        .to_have_text("Group")
        .await
        .unwrap();

    // Micro-slice 5: batch cards with thumbnails
    let cards = page.locator(".batch-card");
    assert_eq!(cards.count().await.unwrap(), 3, "Should have 3 batch cards");

    let first_card = page.locator(".batch-card").nth(0);
    let first_card_text = first_card.text_content().await.unwrap().unwrap();
    assert!(
        first_card_text.contains("thanksgiving-arrival"),
        "First batch should show name 'thanksgiving-arrival'"
    );
    assert!(
        first_card_text.contains("51 files"),
        "First batch should show '51 files'"
    );
    assert!(
        first_card_text.contains("14:00 - 16:30"),
        "First batch should show time range"
    );

    let thumbnails = page.locator(".thumbnail");
    assert!(
        thumbnails.count().await.unwrap() >= 2,
        "Should have at least 2 thumbnails per batch (first and last)"
    );

    let src = page
        .locator("img.thumbnail")
        .nth(0)
        .get_attribute("src")
        .await
        .expect("Failed to get src attribute");
    assert!(
        src.unwrap().starts_with("data:image/jpeg;base64,"),
        "Thumbnail should be base64 data URL"
    );

    browser.close().await.ok();
    server.shutdown().await.expect("Failed to shutdown server");
}

/// Interactive workflow: placeholder names update live over WebSocket as the
/// CLI's interactive naming loop renames each batch.
#[tokio::test]
async fn test_browser_preview_e2e_websocket_updates() {
    let state = three_batch_state(["batch-1", "batch-2", "batch-3"]);

    let server = folio_core::preview::start_preview_server_with_state(state)
        .await
        .expect("Failed to start preview server");
    let preview_url = server.url();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");
    page.goto(&preview_url, None)
        .await
        .expect("Failed to navigate");

    // Placeholder names render first
    expect(page.locator(".batch-card").nth(0))
        .to_contain_text("batch-1")
        .await
        .unwrap();

    // Simulate the CLI interactive loop naming batches one by one; expect()
    // auto-retries, so no sleeps between update and assertion.
    server
        .update_batch_name(0, "thanksgiving-arrival".to_string())
        .await
        .expect("Failed to update batch 1 name");
    expect(page.locator(".batch-card").nth(0))
        .to_contain_text("thanksgiving-arrival")
        .await
        .unwrap();

    server
        .update_batch_name(1, "thanksgiving-dinner".to_string())
        .await
        .expect("Failed to update batch 2 name");
    expect(page.locator(".batch-card").nth(1))
        .to_contain_text("thanksgiving-dinner")
        .await
        .unwrap();

    server
        .update_batch_name(2, "thanksgiving-cleanup".to_string())
        .await
        .expect("Failed to update batch 3 name");
    expect(page.locator(".batch-card").nth(2))
        .to_contain_text("thanksgiving-cleanup")
        .await
        .unwrap();

    // Placeholder fully replaced, card count unchanged
    expect(page.locator(".batch-card").nth(0))
        .not()
        .to_contain_text("batch-1")
        .await
        .unwrap();
    assert_eq!(
        page.locator(".batch-card").count().await.unwrap(),
        3,
        "Should still have 3 batch cards after updates"
    );

    browser.close().await.ok();
    server.shutdown().await.expect("Failed to shutdown server");
}
