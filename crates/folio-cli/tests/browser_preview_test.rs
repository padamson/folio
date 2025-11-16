use folio_core::preview::PreviewState;
use playwright_rs::Playwright;
use std::time::Duration;
use tokio::time::sleep;

/// Comprehensive E2E test using auto-open browser workflow
///
/// Tests microslices 1-6: complete preview functionality with auto-open
#[tokio::test]
async fn test_browser_preview_e2e_auto_open() {
    use folio_core::preview::{
        generate_thumbnail, start_preview_with_browser, BatchPreview, WorkflowStage,
    };
    use std::path::PathBuf;

    // Generate real thumbnail from test fixture
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures/sample-with-exif.jpg");

    let thumbnail = generate_thumbnail(&fixture_path, 100).expect("Failed to generate thumbnail");

    // Create state with 3 batches
    let batches = vec![
        BatchPreview {
            index: 0,
            name: Some("thanksgiving-arrival".to_string()),
            file_count: 51,
            first_photo: Some("DSC_0001.JPG".to_string()),
            last_photo: Some("DSC_0050.JPG".to_string()),
            first_thumbnail: Some(thumbnail.clone()),
            last_thumbnail: Some(thumbnail.clone()),
            time_range: Some("14:00 - 16:30".to_string()),
        },
        BatchPreview {
            index: 1,
            name: Some("thanksgiving-dinner".to_string()),
            file_count: 32,
            first_photo: Some("DSC_0051.JPG".to_string()),
            last_photo: Some("DSC_0082.JPG".to_string()),
            first_thumbnail: Some(thumbnail.clone()),
            last_thumbnail: Some(thumbnail.clone()),
            time_range: Some("17:00 - 19:00".to_string()),
        },
        BatchPreview {
            index: 2,
            name: Some("thanksgiving-cleanup".to_string()),
            file_count: 15,
            first_photo: Some("DSC_0083.JPG".to_string()),
            last_photo: Some("DSC_0097.JPG".to_string()),
            first_thumbnail: Some(thumbnail.clone()),
            last_thumbnail: Some(thumbnail.clone()),
            time_range: Some("20:00 - 21:00".to_string()),
        },
    ];

    let state = PreviewState {
        batch_count: 3,
        current_stage: WorkflowStage::Group,
        batches,
    };

    // MICROSLICE 6: Auto-open browser (one function call)
    let server = start_preview_with_browser(state)
        .await
        .expect("Failed to start preview with browser");

    // Verify browser opened automatically
    assert!(server.has_browser(), "Browser should be attached to server");

    // Get preview URL
    let preview_url = server.url();

    // For testing, we still need to manually access the page via playwright
    // TODO: Determine how to access page from BrowserHandle when needed
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

    sleep(Duration::from_millis(100)).await;

    // Test all microslices (1-6)

    // Micro-slice 1: Verify page contains "Hello Preview"
    let h1 = page.locator("h1").await;
    let text = h1.text_content().await.expect("Failed to get text content");
    assert_eq!(text, Some("Hello Preview".to_string()));

    // Micro-slice 2: Verify page shows batch count
    let batch_count = page.locator(".batch-count").await;
    let count_text = batch_count
        .text_content()
        .await
        .expect("Failed to get batch count text");
    assert_eq!(count_text, Some("3 batches".to_string()));

    // Micro-slice 3: Verify workflow progress displays
    let workflow_progress = page.locator(".workflow-progress").await;
    let progress_text = workflow_progress
        .text_content()
        .await
        .expect("Failed to get workflow progress text");
    assert!(progress_text.is_some());
    assert!(progress_text
        .unwrap()
        .contains("Scan → Group → Name → Review → Copy"));

    // Micro-slice 4: Verify current workflow stage is highlighted
    let current_stage = page.locator(".workflow-stage.active").await;
    let stage_text = current_stage
        .text_content()
        .await
        .expect("Failed to get current stage");
    assert_eq!(stage_text, Some("Group".to_string()));

    // Micro-slice 5: Verify batch cards with thumbnails
    let batch_cards = page.locator(".batch-cards").await;
    assert!(
        batch_cards.count().await.unwrap() > 0,
        "Batch cards container should exist"
    );

    let cards = page.locator(".batch-card").await;
    assert_eq!(cards.count().await.unwrap(), 3, "Should have 3 batch cards");

    let first_card = page.locator(".batch-card").await.nth(0);
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

    let thumbnails = page.locator(".thumbnail").await;
    assert!(
        thumbnails.count().await.unwrap() >= 2,
        "Should have at least 2 thumbnails per batch (first and last)"
    );

    let first_img = page.locator("img.thumbnail").await.nth(0);
    let src = first_img
        .get_attribute("src")
        .await
        .expect("Failed to get src attribute");
    assert!(
        src.unwrap().starts_with("data:image/jpeg;base64,"),
        "Thumbnail should be base64 data URL"
    );

    // Cleanup - close manual browser first, then server (which has its own browser)
    page.close().await.ok();
    browser.close().await.ok();

    // Note: server.shutdown() will also try to close the auto-opened browser
    // This may cause a "Broken pipe" error if the browser is already closed
    // TODO: Improve error handling in shutdown() to ignore already-closed browsers
    server.shutdown().await.ok(); // Use .ok() to ignore shutdown errors for now
}

/// Comprehensive E2E test for browser preview functionality (manual workflow)
///
/// Tests all preview features in a single browser session to reduce overhead.
/// This test uses the manual workflow where browser is launched separately.
#[tokio::test]
async fn test_browser_preview_e2e() {
    // Microslice 5: Create preview state with batch cards and thumbnails
    use folio_core::preview::{generate_thumbnail, BatchPreview, WorkflowStage};
    use std::path::PathBuf;

    // Generate real thumbnail from test fixture
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures/sample-with-exif.jpg");

    let thumbnail = generate_thumbnail(&fixture_path, 100).expect("Failed to generate thumbnail");

    // Create state with 3 batches using PLACEHOLDER names
    // This simulates the CLI workflow where server starts before user provides names
    let batches = vec![
        BatchPreview {
            index: 0,
            name: Some("batch-1".to_string()), // Placeholder
            file_count: 51,
            first_photo: Some("DSC_0001.JPG".to_string()),
            last_photo: Some("DSC_0050.JPG".to_string()),
            first_thumbnail: Some(thumbnail.clone()),
            last_thumbnail: Some(thumbnail.clone()),
            time_range: Some("14:00 - 16:30".to_string()),
        },
        BatchPreview {
            index: 1,
            name: Some("batch-2".to_string()), // Placeholder
            file_count: 32,
            first_photo: Some("DSC_0051.JPG".to_string()),
            last_photo: Some("DSC_0082.JPG".to_string()),
            first_thumbnail: Some(thumbnail.clone()),
            last_thumbnail: Some(thumbnail.clone()),
            time_range: Some("17:00 - 19:00".to_string()),
        },
        BatchPreview {
            index: 2,
            name: Some("batch-3".to_string()), // Placeholder
            file_count: 15,
            first_photo: Some("DSC_0083.JPG".to_string()),
            last_photo: Some("DSC_0097.JPG".to_string()),
            first_thumbnail: Some(thumbnail.clone()),
            last_thumbnail: Some(thumbnail.clone()),
            time_range: Some("20:00 - 21:00".to_string()),
        },
    ];

    let state = PreviewState {
        batch_count: 3,
        current_stage: WorkflowStage::Group,
        batches,
    };

    // Start preview server with state
    let server = folio_core::preview::start_preview_server_with_state(state)
        .await
        .expect("Failed to start preview server");

    let preview_url = server.url();

    // Launch browser
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Navigate to preview URL
    page.goto(&preview_url, None)
        .await
        .expect("Failed to navigate to preview");

    // Wait briefly for page to load
    sleep(Duration::from_millis(100)).await;

    // Micro-slice 1: Verify page contains "Hello Preview"
    let h1 = page.locator("h1").await;
    let text = h1.text_content().await.expect("Failed to get text content");
    assert_eq!(text, Some("Hello Preview".to_string()));

    // Micro-slice 2: Verify page shows batch count
    let batch_count = page.locator(".batch-count").await;
    let count_text = batch_count
        .text_content()
        .await
        .expect("Failed to get batch count text");
    assert_eq!(count_text, Some("3 batches".to_string()));

    // Micro-slice 3: Verify workflow progress displays
    let workflow_progress = page.locator(".workflow-progress").await;
    let progress_text = workflow_progress
        .text_content()
        .await
        .expect("Failed to get workflow progress text");
    assert!(progress_text.is_some());
    assert!(progress_text
        .unwrap()
        .contains("Scan → Group → Name → Review → Copy"));

    // Micro-slice 4: Verify current workflow stage is highlighted
    let current_stage = page.locator(".workflow-stage.active").await;
    let stage_text = current_stage
        .text_content()
        .await
        .expect("Failed to get current stage");
    assert_eq!(stage_text, Some("Group".to_string()));

    // Micro-slice 5: Verify batch cards with thumbnails
    // Verify batch cards container exists
    let batch_cards = page.locator(".batch-cards").await;
    assert!(
        batch_cards.count().await.unwrap() > 0,
        "Batch cards container should exist"
    );

    // Verify we have 3 batch cards
    let cards = page.locator(".batch-card").await;
    assert_eq!(cards.count().await.unwrap(), 3, "Should have 3 batch cards");

    // Verify first batch card content (with PLACEHOLDER name)
    let first_card = page.locator(".batch-card").await.nth(0);
    let first_card_text = first_card.text_content().await.unwrap().unwrap();

    // Check batch name (placeholder)
    assert!(
        first_card_text.contains("batch-1"),
        "First batch should show placeholder name 'batch-1'"
    );

    // Check file count
    assert!(
        first_card_text.contains("51 files"),
        "First batch should show '51 files'"
    );

    // Check time range
    assert!(
        first_card_text.contains("14:00 - 16:30"),
        "First batch should show time range"
    );

    // Verify thumbnails are displayed as images
    let thumbnails = page.locator(".thumbnail").await;
    assert!(
        thumbnails.count().await.unwrap() >= 2,
        "Should have at least 2 thumbnails per batch (first and last)"
    );

    // Verify img tags have base64 data URLs
    let first_img = page.locator("img.thumbnail").await.nth(0);
    let src = first_img
        .get_attribute("src")
        .await
        .expect("Failed to get src attribute");
    assert!(
        src.unwrap().starts_with("data:image/jpeg;base64,"),
        "Thumbnail should be base64 data URL"
    );

    // Phase 2: Test WebSocket real-time updates simulating CLI interactive workflow
    // Simulate user naming batches one-by-one in interactive mode

    // User names batch 1
    server
        .update_batch_name(0, "thanksgiving-arrival".to_string())
        .await
        .expect("Failed to update batch 1 name");

    sleep(Duration::from_millis(100)).await;

    let card_0 = page.locator(".batch-card").await.nth(0);
    let text_0 = card_0.text_content().await.unwrap().unwrap();
    assert!(
        text_0.contains("thanksgiving-arrival"),
        "Batch 1 should update to 'thanksgiving-arrival' via WebSocket"
    );

    // User names batch 2
    server
        .update_batch_name(1, "thanksgiving-dinner".to_string())
        .await
        .expect("Failed to update batch 2 name");

    sleep(Duration::from_millis(100)).await;

    let card_1 = page.locator(".batch-card").await.nth(1);
    let text_1 = card_1.text_content().await.unwrap().unwrap();
    assert!(
        text_1.contains("thanksgiving-dinner"),
        "Batch 2 should update to 'thanksgiving-dinner' via WebSocket"
    );

    // User names batch 3
    server
        .update_batch_name(2, "thanksgiving-cleanup".to_string())
        .await
        .expect("Failed to update batch 3 name");

    sleep(Duration::from_millis(100)).await;

    let card_2 = page.locator(".batch-card").await.nth(2);
    let text_2 = card_2.text_content().await.unwrap().unwrap();
    assert!(
        text_2.contains("thanksgiving-cleanup"),
        "Batch 3 should update to 'thanksgiving-cleanup' via WebSocket"
    );

    // Verify all three batches now show user-provided names (not placeholders)
    let all_cards = page.locator(".batch-card").await;
    assert_eq!(
        all_cards.count().await.unwrap(),
        3,
        "Should still have 3 batch cards after updates"
    );

    // Verify first batch no longer shows placeholder
    let final_card_0_text = page
        .locator(".batch-card")
        .await
        .nth(0)
        .text_content()
        .await
        .unwrap()
        .unwrap();
    assert!(
        !final_card_0_text.contains("batch-1"),
        "Batch 1 should no longer show placeholder 'batch-1'"
    );
    assert!(
        final_card_0_text.contains("thanksgiving-arrival"),
        "Batch 1 should show final name 'thanksgiving-arrival'"
    );

    // Cleanup
    browser.close().await.ok();
    server.shutdown().await.expect("Failed to shutdown server");
}
