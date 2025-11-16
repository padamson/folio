//! Browser preview server for ingestion workflow
//!
//! Provides a lightweight HTTP server that serves a live preview dashboard
//! during media ingestion. The dashboard shows batch groupings, thumbnails,
//! and workflow progress in real-time.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State as AxumState, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, GenericImageView, ImageReader};
use playwright_core::protocol::Browser;
use playwright_rs::Playwright;
use std::io::Cursor;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;

/// Workflow stages during ingestion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStage {
    Scan,
    Group,
    Name,
    Review,
    Copy,
}

impl WorkflowStage {
    /// Get the display name of the workflow stage
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkflowStage::Scan => "Scan",
            WorkflowStage::Group => "Group",
            WorkflowStage::Name => "Name",
            WorkflowStage::Review => "Review",
            WorkflowStage::Copy => "Copy",
        }
    }
}

/// Information about a single batch for preview
///
/// Contains rich metadata about a batch including thumbnails, file counts,
/// and time ranges for display in the browser preview dashboard.
#[derive(Debug, Clone)]
pub struct BatchPreview {
    /// Batch index (0-based)
    pub index: usize,
    /// Batch name (if named, otherwise None)
    pub name: Option<String>,
    /// Number of files in this batch
    pub file_count: usize,
    /// First photo filename (for thumbnail)
    pub first_photo: Option<String>,
    /// Last photo filename (for thumbnail)
    pub last_photo: Option<String>,
    /// First photo thumbnail (JPEG bytes)
    pub first_thumbnail: Option<Vec<u8>>,
    /// Last photo thumbnail (JPEG bytes)
    pub last_thumbnail: Option<Vec<u8>>,
    /// Time range string (e.g., "14:00 - 16:30")
    pub time_range: Option<String>,
}

/// Preview state shared between server and updates
///
/// Contains information about the ingestion workflow progress,
/// batch groupings, and thumbnails.
#[derive(Debug, Clone)]
pub struct PreviewState {
    /// Number of batches detected
    pub batch_count: usize,
    /// Current workflow stage
    pub current_stage: WorkflowStage,
    /// Detailed batch information
    pub batches: Vec<BatchPreview>,
}

impl PreviewState {
    /// Create a new preview state with the given batch count
    ///
    /// Defaults to WorkflowStage::Group
    ///
    /// # Example
    ///
    /// ```
    /// use folio_core::preview::{PreviewState, WorkflowStage};
    ///
    /// let state = PreviewState::new(3);
    /// assert_eq!(state.batch_count, 3);
    /// assert_eq!(state.current_stage, WorkflowStage::Group);
    /// ```
    pub fn new(batch_count: usize) -> Self {
        Self {
            batch_count,
            current_stage: WorkflowStage::Group,
            batches: Vec::new(),
        }
    }
}

/// Handle to keep browser instance alive
///
/// The browser will close when this handle is dropped.
pub struct BrowserHandle {
    browser: Browser,
}

impl BrowserHandle {
    /// Check if browser is still open
    ///
    /// For now, just returns true (assumes browser is open).
    /// TODO: Add actual check when playwright-rust supports it.
    /// See: https://github.com/padamson/playwright-rust/issues/2
    pub fn is_open(&self) -> bool {
        // playwright-rust doesn't provide a way to check if browser is closed
        // For now, assume it's open if we have the handle
        true
    }

    /// Close the browser gracefully
    pub async fn close(self) -> Result<(), String> {
        self.browser
            .close()
            .await
            .map_err(|e| format!("Failed to close browser: {}", e))
    }
}

/// Open the preview URL in the default browser
///
/// Uses playwright-rs to launch a Chromium browser instance and navigate to the preview URL.
/// The browser stays open in normal (non-headless) mode for user visibility.
///
/// # Arguments
///
/// * `url` - The preview server URL to open
///
/// # Returns
///
/// Browser handle that must be kept alive
///
/// # Errors
///
/// Returns error if:
/// - playwright-rs initialization fails
/// - Browser launch fails
/// - Navigation to URL fails
///
/// # Example
///
/// ```no_run
/// use folio_core::preview::open_browser;
///
/// # tokio_test::block_on(async {
/// let handle = open_browser("http://127.0.0.1:8080").await?;
/// assert!(handle.is_open());
/// handle.close().await?;
/// # Ok::<(), String>(())
/// # });
/// ```
pub async fn open_browser(url: &str) -> Result<BrowserHandle, String> {
    // Launch playwright
    let playwright = Playwright::launch()
        .await
        .map_err(|e| format!("Failed to launch playwright: {}", e))?;

    // Launch Chromium in non-headless mode (user-visible)
    // TODO: playwright-rust doesn't expose headless option directly in launch()
    // See: https://github.com/padamson/playwright-rust/issues/1
    let browser = playwright
        .chromium()
        .launch()
        .await
        .map_err(|e| format!("Failed to launch browser: {}", e))?;

    // Create a new page and navigate to the URL
    let page = browser
        .new_page()
        .await
        .map_err(|e| format!("Failed to create page: {}", e))?;

    page.goto(url, None)
        .await
        .map_err(|e| format!("Failed to navigate to {}: {}", url, e))?;

    // Return handle with browser (page stays open with browser)
    Ok(BrowserHandle { browser })
}

/// Shared application state for WebSocket updates
///
/// Contains both the preview state and a broadcast channel for sending
/// real-time updates to connected WebSocket clients.
#[derive(Clone)]
struct AppState {
    preview: Arc<RwLock<PreviewState>>,
    tx: broadcast::Sender<String>,
}

/// Preview server handle
///
/// Provides access to the server URL and methods to update state and shutdown.
pub struct PreviewServer {
    addr: SocketAddr,
    handle: JoinHandle<()>,
    browser: Option<BrowserHandle>,
    state: Option<Arc<RwLock<PreviewState>>>,
    tx: Option<broadcast::Sender<String>>,
}

impl PreviewServer {
    /// Get the URL of the preview server
    ///
    /// # Example
    ///
    /// ```
    /// use folio_core::preview::start_preview_server;
    ///
    /// # tokio_test::block_on(async {
    /// let server = start_preview_server().await?;
    /// let url = server.url();
    /// assert!(url.starts_with("http://127.0.0.1:"));
    /// server.shutdown().await?;
    /// # Ok::<(), String>(())
    /// # });
    /// ```
    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Check if server has browser attached
    pub fn has_browser(&self) -> bool {
        self.browser.is_some()
    }

    /// Update a batch name in the preview state
    ///
    /// This method updates the shared state, but does NOT yet trigger
    /// WebSocket broadcast (Phase 2 WebSocket functionality pending).
    ///
    /// # Arguments
    ///
    /// * `index` - The batch index (0-based)
    /// * `name` - The new name for the batch
    ///
    /// # Returns
    ///
    /// Ok if the batch was updated successfully
    /// Err if the batch index is out of bounds or no state is available
    ///
    /// # Example
    ///
    /// ```
    /// use folio_core::preview::{start_preview_server_with_state, PreviewState, BatchPreview};
    ///
    /// # tokio_test::block_on(async {
    /// let batches = vec![
    ///     BatchPreview {
    ///         index: 0,
    ///         name: Some("batch1".to_string()),
    ///         file_count: 10,
    ///         first_photo: None,
    ///         last_photo: None,
    ///         first_thumbnail: None,
    ///         last_thumbnail: None,
    ///         time_range: None,
    ///     },
    /// ];
    ///
    /// let state = PreviewState {
    ///     batch_count: 1,
    ///     current_stage: folio_core::preview::WorkflowStage::Group,
    ///     batches,
    /// };
    ///
    /// let server = start_preview_server_with_state(state).await?;
    ///
    /// // Update batch name
    /// server.update_batch_name(0, "new-name".to_string()).await?;
    ///
    /// server.shutdown().await?;
    /// # Ok::<(), String>(())
    /// # });
    /// ```
    pub async fn update_batch_name(&self, index: usize, name: String) -> Result<(), String> {
        // Get the shared state (if available)
        let state = self
            .state
            .as_ref()
            .ok_or("No state available - server started without state")?;

        // Lock and update the state
        let mut state = state.write().await;

        // Find the batch by index and update its name
        if index >= state.batches.len() {
            return Err(format!(
                "Batch index {} out of bounds (have {} batches)",
                index,
                state.batches.len()
            ));
        }

        state.batches[index].name = Some(name.clone());

        // Drop the write lock before broadcasting
        drop(state);

        // Broadcast WebSocket update (Phase 2)
        if let Some(tx) = &self.tx {
            let update = serde_json::json!({
                "type": "batch_name_update",
                "index": index,
                "name": name
            });
            let message = serde_json::to_string(&update)
                .map_err(|e| format!("Failed to serialize update: {}", e))?;

            // Ignore errors if no clients are connected
            let _ = tx.send(message);
        }

        Ok(())
    }

    /// Shutdown the preview server and close browser if attached
    ///
    /// Closes the browser first (if present), then shuts down the server.
    ///
    /// # Example
    ///
    /// ```
    /// use folio_core::preview::start_preview_server;
    ///
    /// # tokio_test::block_on(async {
    /// let server = start_preview_server().await?;
    /// server.shutdown().await?;
    /// # Ok::<(), String>(())
    /// # });
    /// ```
    pub async fn shutdown(self) -> Result<(), String> {
        // Close browser first if present
        if let Some(browser) = self.browser {
            browser.close().await?;
        }

        // Shutdown server
        self.handle.abort();
        Ok(())
    }
}

/// Start the preview server (without state)
///
/// Starts a lightweight HTTP server on a random available port and returns
/// a handle to interact with it. This version has no state.
///
/// # Example
///
/// ```
/// use folio_core::preview::start_preview_server;
///
/// # tokio_test::block_on(async {
/// let server = start_preview_server().await?;
///
/// // Verify URL format
/// let url = server.url();
/// assert!(url.starts_with("http://127.0.0.1:"));
/// assert!(url.contains(":"));
///
/// // Cleanup
/// server.shutdown().await?;
/// # Ok::<(), String>(())
/// # });
/// ```
pub async fn start_preview_server() -> Result<PreviewServer, String> {
    // Create router with a single route serving "Hello Preview"
    let app = Router::new().route("/", get(hello_handler));

    // Bind to random available port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind: {}", e))?;

    let addr = listener
        .local_addr()
        .map_err(|e| format!("Failed to get local address: {}", e))?;

    // Spawn server in background
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("Server failed to start");
    });

    Ok(PreviewServer {
        addr,
        handle,
        browser: None,
        state: None,
        tx: None,
    })
}

/// Start the preview server with state
///
/// Starts a lightweight HTTP server with preview state that will be displayed
/// in the browser dashboard.
///
/// # Example
///
/// ```
/// use folio_core::preview::{start_preview_server_with_state, PreviewState};
///
/// # tokio_test::block_on(async {
/// let state = PreviewState::new(3);
/// let server = start_preview_server_with_state(state).await?;
///
/// // Verify server starts and URL is valid
/// let url = server.url();
/// assert!(url.starts_with("http://127.0.0.1:"));
///
/// server.shutdown().await?;
/// # Ok::<(), String>(())
/// # });
/// ```
pub async fn start_preview_server_with_state(state: PreviewState) -> Result<PreviewServer, String> {
    // Wrap state in Arc<RwLock<>> for shared access
    let shared_state = Arc::new(RwLock::new(state));

    // Create broadcast channel for WebSocket updates
    let (tx, _rx) = broadcast::channel::<String>(100);

    // Create app state with both preview state and broadcast channel
    let app_state = AppState {
        preview: shared_state.clone(),
        tx: tx.clone(),
    };

    // Create router with WebSocket route
    let app = Router::new()
        .route("/", get(preview_handler_with_ws))
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    // Bind to random available port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind: {}", e))?;

    let addr = listener
        .local_addr()
        .map_err(|e| format!("Failed to get local address: {}", e))?;

    // Spawn server in background
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("Server failed to start");
    });

    Ok(PreviewServer {
        addr,
        handle,
        browser: None,
        state: Some(shared_state),
        tx: Some(tx),
    })
}

/// Start preview server and automatically open browser
///
/// Convenience function that combines server startup and browser launching.
/// The browser opens in normal (non-headless) mode for user visibility.
///
/// # Arguments
///
/// * `state` - Preview state to display
///
/// # Returns
///
/// PreviewServer with browser handle attached
///
/// # Example
///
/// ```no_run
/// use folio_core::preview::{start_preview_with_browser, PreviewState};
///
/// # tokio_test::block_on(async {
/// let state = PreviewState::new(3);
/// let server = start_preview_with_browser(state).await?;
///
/// // Server is running with browser already open
/// assert!(server.has_browser());
///
/// // Cleanup (closes both browser and server)
/// server.shutdown().await?;
/// # Ok::<(), String>(())
/// # });
/// ```
pub async fn start_preview_with_browser(state: PreviewState) -> Result<PreviewServer, String> {
    // Start server first
    let mut server = start_preview_server_with_state(state).await?;

    // Open browser and navigate to server URL
    let browser = open_browser(&server.url()).await?;

    // Attach browser to server
    server.browser = Some(browser);

    Ok(server)
}

/// Handler for the root route - serves minimal HTML with "Hello Preview"
async fn hello_handler() -> axum::response::Html<String> {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Folio Ingestion Preview</title>
    <meta charset="UTF-8">
</head>
<body>
    <h1>Hello Preview</h1>
</body>
</html>
"#;
    axum::response::Html(html.to_string())
}

/// Handler for WebSocket upgrades
///
/// Accepts WebSocket connections and broadcasts state updates to connected clients.
async fn ws_handler(
    ws: WebSocketUpgrade,
    AxumState(state): AxumState<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle a WebSocket connection
///
/// Subscribes to the broadcast channel and forwards messages to the client.
async fn handle_socket(mut socket: WebSocket, state: AppState) {
    // Subscribe to broadcast channel
    let mut rx = state.tx.subscribe();

    // Forward messages from broadcast to WebSocket
    while let Ok(msg) = rx.recv().await {
        if socket.send(Message::Text(msg.into())).await.is_err() {
            // Client disconnected
            break;
        }
    }
}

/// Handler for preview with WebSocket support
///
/// Like preview_handler but includes JavaScript WebSocket client.
async fn preview_handler_with_ws(
    AxumState(state): AxumState<AppState>,
) -> axum::response::Html<String> {
    let preview_state = state.preview.read().await;

    // Generate workflow stages HTML
    let stages = [
        WorkflowStage::Scan,
        WorkflowStage::Group,
        WorkflowStage::Name,
        WorkflowStage::Review,
        WorkflowStage::Copy,
    ];

    let workflow_html: String = stages
        .iter()
        .map(|stage| {
            let active_class = if *stage == preview_state.current_stage {
                " active"
            } else {
                ""
            };
            format!(
                r#"<span class="workflow-stage{}">{}</span>"#,
                active_class,
                stage.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join(" → ");

    // Generate batch cards HTML
    let batch_cards_html: String = preview_state
        .batches
        .iter()
        .map(|batch| {
            // Generate batch name (use index if no name provided)
            let batch_name = batch
                .name
                .as_ref()
                .map(|n| format!("Batch {}: {}", batch.index + 1, n))
                .unwrap_or_else(|| format!("Batch {}", batch.index + 1));

            // Generate thumbnails HTML
            let mut thumbnails_html = String::new();

            if let Some(first_thumb) = &batch.first_thumbnail {
                let data_url = encode_thumbnail_base64(first_thumb);
                thumbnails_html.push_str(&format!(
                    r#"<img src="{}" class="thumbnail first" alt="First photo" />"#,
                    data_url
                ));
            }

            if let Some(last_thumb) = &batch.last_thumbnail {
                let data_url = encode_thumbnail_base64(last_thumb);
                thumbnails_html.push_str(&format!(
                    r#"<img src="{}" class="thumbnail last" alt="Last photo" />"#,
                    data_url
                ));
            }

            // Generate time range HTML
            let time_range_html = batch
                .time_range
                .as_ref()
                .map(|tr| format!(r#"<span class="time-range">{}</span>"#, tr))
                .unwrap_or_default();

            format!(
                r#"<div class="batch-card" data-batch-index="{}">
    <h3 class="batch-name">{}</h3>
    <div class="batch-stats">
      <span class="file-count">{} files</span>
      {}
    </div>
    <div class="thumbnails">
      {}
    </div>
  </div>"#,
                batch.index, batch_name, batch.file_count, time_range_html, thumbnails_html
            )
        })
        .collect::<Vec<_>>()
        .join("\n  ");

    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Folio Ingestion Preview</title>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 2rem;
            color: #333;
        }}

        .container {{
            max-width: 1200px;
            margin: 0 auto;
        }}

        h1 {{
            color: white;
            font-size: 2.5rem;
            font-weight: 700;
            margin-bottom: 1rem;
            text-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
        }}

        .batch-count {{
            color: rgba(255, 255, 255, 0.9);
            font-size: 1.2rem;
            margin-bottom: 2rem;
            font-weight: 500;
        }}

        /* Workflow Progress */
        .workflow-progress {{
            background: white;
            border-radius: 12px;
            padding: 1.5rem;
            margin-bottom: 2rem;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
            display: flex;
            align-items: center;
            justify-content: center;
            flex-wrap: wrap;
            gap: 0.5rem;
        }}

        .workflow-stage {{
            padding: 0.5rem 1rem;
            border-radius: 6px;
            background: #f3f4f6;
            color: #6b7280;
            font-weight: 500;
            transition: all 0.3s ease;
        }}

        .workflow-stage.active {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            box-shadow: 0 2px 8px rgba(102, 126, 234, 0.4);
            transform: scale(1.05);
        }}

        /* Batch Cards Grid */
        .batch-cards {{
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
            gap: 1.5rem;
            margin-top: 2rem;
        }}

        .batch-card {{
            background: white;
            border-radius: 12px;
            padding: 1.5rem;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
            transition: transform 0.2s ease, box-shadow 0.2s ease;
        }}

        .batch-card:hover {{
            transform: translateY(-4px);
            box-shadow: 0 8px 12px rgba(0, 0, 0, 0.15);
        }}

        .batch-card h3 {{
            color: #1f2937;
            font-size: 1.25rem;
            margin-bottom: 1rem;
            font-weight: 600;
        }}

        .batch-stats {{
            display: flex;
            gap: 1rem;
            margin-bottom: 1rem;
            flex-wrap: wrap;
        }}

        .file-count {{
            background: #eff6ff;
            color: #1e40af;
            padding: 0.25rem 0.75rem;
            border-radius: 6px;
            font-size: 0.875rem;
            font-weight: 600;
        }}

        .time-range {{
            background: #f0fdf4;
            color: #166534;
            padding: 0.25rem 0.75rem;
            border-radius: 6px;
            font-size: 0.875rem;
            font-weight: 600;
        }}

        /* Thumbnails */
        .thumbnails {{
            display: flex;
            gap: 0.75rem;
            margin-top: 1rem;
        }}

        .thumbnail {{
            width: 120px;
            height: 120px;
            object-fit: cover;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
            transition: transform 0.2s ease;
        }}

        .thumbnail:hover {{
            transform: scale(1.05);
        }}

        /* Responsive Design */
        @media (max-width: 768px) {{
            body {{
                padding: 1rem;
            }}

            h1 {{
                font-size: 2rem;
            }}

            .batch-cards {{
                grid-template-columns: 1fr;
            }}

            .workflow-progress {{
                font-size: 0.875rem;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Hello Preview</h1>
        <div class="batch-count">{} batches</div>
        <div class="workflow-progress">{}</div>
        <div class="batch-cards">
  {}
        </div>
    </div>

    <script>
        // WebSocket connection for real-time updates
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${{protocol}}//${{window.location.host}}/ws`;
        let ws;
        let reconnectInterval = 1000;

        function connect() {{
            ws = new WebSocket(wsUrl);

            ws.onopen = () => {{
                console.log('WebSocket connected');
                reconnectInterval = 1000; // Reset reconnect interval
            }};

            ws.onmessage = (event) => {{
                try {{
                    const update = JSON.parse(event.data);
                    handleUpdate(update);
                }} catch (e) {{
                    console.error('Failed to parse update:', e);
                }}
            }};

            ws.onclose = () => {{
                console.log('WebSocket closed, reconnecting...');
                setTimeout(connect, reconnectInterval);
                reconnectInterval = Math.min(reconnectInterval * 2, 30000); // Exponential backoff
            }};

            ws.onerror = (error) => {{
                console.error('WebSocket error:', error);
                ws.close();
            }};
        }}

        function handleUpdate(update) {{
            if (update.type === 'batch_name_update') {{
                updateBatchName(update.index, update.name);
            }}
        }}

        function updateBatchName(index, name) {{
            const card = document.querySelector(`.batch-card[data-batch-index="${{index}}"]`);
            if (card) {{
                const nameElement = card.querySelector('.batch-name');
                if (nameElement) {{
                    // Update the text
                    nameElement.textContent = `Batch ${{index + 1}}: ${{name}}`;

                    // Add visual feedback (brief highlight)
                    card.style.transition = 'background-color 0.3s ease';
                    card.style.backgroundColor = '#fef3c7';  // Yellow highlight
                    setTimeout(() => {{
                        card.style.backgroundColor = 'white';
                    }}, 300);
                }}
            }}
        }}

        // Connect on page load
        connect();
    </script>
</body>
</html>
"#,
        preview_state.batch_count, workflow_html, batch_cards_html
    );

    axum::response::Html(html)
}

/// JPEG quality for thumbnail generation (0-100)
const THUMBNAIL_JPEG_QUALITY: u8 = 85;

/// Generate a thumbnail from an image file
///
/// Creates a JPEG thumbnail from the input image, resizing to fit within
/// `max_size` × `max_size` while preserving aspect ratio. Images smaller
/// than `max_size` are not upscaled.
///
/// # Arguments
///
/// * `image_path` - Path to the source image file (JPEG, PNG, etc.)
/// * `max_size` - Maximum dimension (width or height) for the thumbnail
///
/// # Returns
///
/// JPEG-encoded thumbnail as bytes
///
/// # Errors
///
/// Returns error if:
/// - File does not exist or is not readable
/// - File is not a valid image format
/// - Image processing fails
///
/// # Example
///
/// ```
/// use folio_core::preview::generate_thumbnail;
/// use std::path::PathBuf;
///
/// // Build path to test fixture using CARGO_MANIFEST_DIR
/// let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
///     .parent().unwrap()
///     .parent().unwrap()
///     .join("test-data/fixtures/sample-with-exif.jpg");
///
/// // Generate a 200x200 thumbnail from test fixture
/// let thumbnail = generate_thumbnail(&fixture_path, 200)?;
///
/// // Verify we got JPEG bytes
/// assert!(!thumbnail.is_empty());
/// assert_eq!(thumbnail[0], 0xFF); // JPEG magic byte
/// assert_eq!(thumbnail[1], 0xD8); // JPEG magic byte
///
/// // Decode to verify dimensions
/// let img = image::load_from_memory(&thumbnail)?;
/// assert!(img.width() <= 200);
/// assert!(img.height() <= 200);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn generate_thumbnail(image_path: &Path, max_size: u32) -> anyhow::Result<Vec<u8>> {
    // Load the image
    let img = ImageReader::open(image_path)
        .map_err(|e| anyhow::anyhow!("Failed to open image: {}", e))?
        .decode()
        .map_err(|e| anyhow::anyhow!("Failed to decode image: {}", e))?;

    // Calculate thumbnail dimensions (don't upscale)
    let (width, height) = img.dimensions();
    let thumbnail = if width > max_size || height > max_size {
        // Resize to fit within max_size
        img.thumbnail(max_size, max_size)
    } else {
        // Don't upscale - keep original size
        img
    };

    // Encode as JPEG
    encode_as_jpeg(&thumbnail, THUMBNAIL_JPEG_QUALITY)
}

/// Encode thumbnail bytes as base64 data URL for HTML embedding
///
/// Converts JPEG-encoded thumbnail bytes into a base64-encoded data URL
/// that can be embedded directly in HTML `<img>` tags.
///
/// # Arguments
///
/// * `jpeg_bytes` - JPEG-encoded thumbnail data
///
/// # Returns
///
/// Base64-encoded data URL with format: `data:image/jpeg;base64,<encoded data>`
///
/// # Example
///
/// ```
/// use folio_core::preview::encode_thumbnail_base64;
///
/// // Minimal JPEG bytes (magic header)
/// let jpeg_bytes = vec![0xFF, 0xD8, 0xFF];
///
/// let data_url = encode_thumbnail_base64(&jpeg_bytes);
///
/// // Verify format
/// assert!(data_url.starts_with("data:image/jpeg;base64,"));
/// assert!(data_url.len() > "data:image/jpeg;base64,".len());
/// ```
pub fn encode_thumbnail_base64(jpeg_bytes: &[u8]) -> String {
    use base64::{engine::general_purpose, Engine as _};
    let encoded = general_purpose::STANDARD.encode(jpeg_bytes);
    format!("data:image/jpeg;base64,{}", encoded)
}

/// Encode a DynamicImage as JPEG bytes
fn encode_as_jpeg(img: &DynamicImage, quality: u8) -> anyhow::Result<Vec<u8>> {
    let mut jpeg_bytes = Vec::new();
    let mut cursor = Cursor::new(&mut jpeg_bytes);

    // Convert to RGB8 (JPEG doesn't support alpha channel)
    let rgb_img = img.to_rgb8();

    // Create JPEG encoder with specified quality
    let mut encoder = JpegEncoder::new_with_quality(&mut cursor, quality);

    // Encode the image
    encoder
        .encode(
            rgb_img.as_raw(),
            rgb_img.width(),
            rgb_img.height(),
            image::ColorType::Rgb8.into(),
        )
        .map_err(|e| anyhow::anyhow!("Failed to encode JPEG: {}", e))?;

    Ok(jpeg_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Helper to get path to test fixtures from workspace root
    fn fixture_path(filename: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test-data/fixtures")
            .join(filename)
    }

    #[tokio::test]
    async fn test_preview_state_new() {
        let state = PreviewState::new(5);
        assert_eq!(state.batch_count, 5);
    }

    #[tokio::test]
    async fn test_start_preview_server() {
        let server = start_preview_server()
            .await
            .expect("Failed to start server");

        // Verify URL format
        let url = server.url();
        assert!(url.starts_with("http://127.0.0.1:"));

        // Cleanup
        server.shutdown().await.expect("Failed to shutdown");
    }

    #[tokio::test]
    async fn test_start_preview_server_with_state() {
        let state = PreviewState::new(3);
        let server = start_preview_server_with_state(state)
            .await
            .expect("Failed to start server");

        // Verify URL format
        let url = server.url();
        assert!(url.starts_with("http://127.0.0.1:"));

        // Cleanup
        server.shutdown().await.expect("Failed to shutdown");
    }

    #[tokio::test]
    async fn test_hello_handler_returns_html() {
        let response = hello_handler().await;
        let html = response.0;

        assert!(html.contains("<h1>Hello Preview</h1>"));
        assert!(html.contains("Folio Ingestion Preview"));
    }

    #[tokio::test]
    async fn test_preview_handler_shows_batch_count() {
        let state = PreviewState::new(3);
        let shared_state = Arc::new(RwLock::new(state));
        let (tx, _rx) = broadcast::channel::<String>(100);
        let app_state = AppState {
            preview: shared_state,
            tx,
        };

        let response = preview_handler_with_ws(AxumState(app_state)).await;
        let html = response.0;

        assert!(html.contains("<h1>Hello Preview</h1>"));
        assert!(html.contains("3 batches"));
        assert!(html.contains("class=\"batch-count\""));
    }

    // ========== Thumbnail Generation Tests ==========

    #[test]
    fn test_generate_thumbnail_resizes_large_image() {
        // Use sample-with-exif.jpg (100x100) and resize to 50x50
        let input = fixture_path("sample-with-exif.jpg");
        let max_size = 50;

        let thumbnail_bytes =
            generate_thumbnail(&input, max_size).expect("Failed to generate thumbnail");

        // Verify we got JPEG bytes
        assert!(!thumbnail_bytes.is_empty());

        // Decode thumbnail to verify dimensions
        let img = image::load_from_memory(&thumbnail_bytes).expect("Failed to decode thumbnail");

        // Should be resized to fit within 50x50
        assert!(img.width() <= max_size);
        assert!(img.height() <= max_size);
    }

    #[test]
    fn test_generate_thumbnail_preserves_aspect_ratio() {
        // Use sample-with-exif.jpg and verify aspect ratio is preserved
        let input = fixture_path("sample-with-exif.jpg");
        let max_size = 50;

        let thumbnail_bytes =
            generate_thumbnail(&input, max_size).expect("Failed to generate thumbnail");

        let img = image::load_from_memory(&thumbnail_bytes).expect("Failed to decode thumbnail");

        // For a square image (100x100), thumbnail should also be square
        assert_eq!(img.width(), img.height(), "Aspect ratio not preserved");
    }

    #[test]
    fn test_generate_thumbnail_doesnt_upscale() {
        // Use minimal.jpg (1x1) and verify it doesn't get upscaled
        let input = fixture_path("minimal.jpg");
        let max_size = 200;

        let thumbnail_bytes =
            generate_thumbnail(&input, max_size).expect("Failed to generate thumbnail");

        let img = image::load_from_memory(&thumbnail_bytes).expect("Failed to decode thumbnail");

        // Should NOT be upscaled - dimensions should stay 1x1
        assert_eq!(img.width(), 1, "Image was upscaled (width)");
        assert_eq!(img.height(), 1, "Image was upscaled (height)");
    }

    #[test]
    fn test_generate_thumbnail_invalid_file() {
        // Use a non-image file (corrupted.jpg is not a valid image)
        let input = fixture_path("corrupted.jpg");
        let max_size = 200;

        let result = generate_thumbnail(&input, max_size);

        // Should return an error
        assert!(result.is_err(), "Expected error for invalid image file");
    }

    #[test]
    fn test_generate_thumbnail_missing_file() {
        // Use a file that doesn't exist
        let input = fixture_path("nonexistent.jpg");
        let max_size = 200;

        let result = generate_thumbnail(&input, max_size);

        // Should return an error
        assert!(result.is_err(), "Expected error for missing file");
    }

    #[test]
    fn test_generate_thumbnail_returns_jpeg_bytes() {
        // Verify that output is valid JPEG format
        let input = fixture_path("sample-with-exif.jpg");
        let max_size = 100;

        let thumbnail_bytes =
            generate_thumbnail(&input, max_size).expect("Failed to generate thumbnail");

        // Check JPEG magic bytes (FF D8 FF)
        assert!(thumbnail_bytes.len() > 3, "Thumbnail too small to be JPEG");
        assert_eq!(thumbnail_bytes[0], 0xFF, "Not a JPEG (byte 0)");
        assert_eq!(thumbnail_bytes[1], 0xD8, "Not a JPEG (byte 1)");
        assert_eq!(thumbnail_bytes[2], 0xFF, "Not a JPEG (byte 2)");
    }

    // ========== Batch Cards Tests (Slice 3c, microslice 5) ==========

    #[test]
    fn test_batch_preview_new() {
        // Create a BatchPreview with all required fields
        let thumbnail = vec![0xFF, 0xD8, 0xFF]; // Minimal JPEG bytes

        let batch = BatchPreview {
            index: 0,
            name: Some("thanksgiving-arrival".to_string()),
            file_count: 51,
            first_photo: Some("DSC_0001.JPG".to_string()),
            last_photo: Some("DSC_0050.JPG".to_string()),
            first_thumbnail: Some(thumbnail.clone()),
            last_thumbnail: Some(thumbnail.clone()),
            time_range: Some("14:00 - 16:30".to_string()),
        };

        // Verify fields
        assert_eq!(batch.index, 0);
        assert_eq!(batch.name, Some("thanksgiving-arrival".to_string()));
        assert_eq!(batch.file_count, 51);
        assert_eq!(batch.first_photo, Some("DSC_0001.JPG".to_string()));
        assert_eq!(batch.last_photo, Some("DSC_0050.JPG".to_string()));
        assert!(batch.first_thumbnail.is_some());
        assert!(batch.last_thumbnail.is_some());
        assert_eq!(batch.time_range, Some("14:00 - 16:30".to_string()));
    }

    #[test]
    fn test_encode_thumbnail_base64() {
        // Encode minimal JPEG bytes as base64 data URL
        let jpeg_bytes = vec![0xFF, 0xD8, 0xFF]; // Minimal JPEG

        let data_url = encode_thumbnail_base64(&jpeg_bytes);

        // Should not be empty
        assert!(!data_url.is_empty());

        // Should be base64 encoded
        assert!(data_url.len() > jpeg_bytes.len());
    }

    #[test]
    fn test_encode_thumbnail_base64_format() {
        // Verify the data URL has correct prefix
        let jpeg_bytes = vec![0xFF, 0xD8, 0xFF];

        let data_url = encode_thumbnail_base64(&jpeg_bytes);

        // Should start with "data:image/jpeg;base64,"
        assert!(
            data_url.starts_with("data:image/jpeg;base64,"),
            "Data URL doesn't have correct prefix"
        );

        // Should have base64 content after prefix
        let base64_part = data_url.strip_prefix("data:image/jpeg;base64,").unwrap();
        assert!(!base64_part.is_empty());
    }

    #[test]
    fn test_preview_state_with_batches() {
        // Create a PreviewState with batches
        let thumbnail = vec![0xFF, 0xD8, 0xFF];

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
        ];

        let state = PreviewState {
            batch_count: 2,
            current_stage: WorkflowStage::Name,
            batches,
        };

        // Verify state
        assert_eq!(state.batch_count, 2);
        assert_eq!(state.current_stage, WorkflowStage::Name);
        assert_eq!(state.batches.len(), 2);
        assert_eq!(state.batches[0].index, 0);
        assert_eq!(state.batches[1].index, 1);
    }

    #[tokio::test]
    async fn test_preview_handler_renders_batch_cards() {
        // Create state with batches
        let thumbnail = vec![0xFF, 0xD8, 0xFF];

        let batches = vec![BatchPreview {
            index: 0,
            name: Some("thanksgiving-arrival".to_string()),
            file_count: 51,
            first_photo: Some("DSC_0001.JPG".to_string()),
            last_photo: Some("DSC_0050.JPG".to_string()),
            first_thumbnail: Some(thumbnail.clone()),
            last_thumbnail: Some(thumbnail.clone()),
            time_range: Some("14:00 - 16:30".to_string()),
        }];

        let state = PreviewState {
            batch_count: 1,
            current_stage: WorkflowStage::Name,
            batches,
        };

        let shared_state = Arc::new(RwLock::new(state));
        let (tx, _rx) = broadcast::channel::<String>(100);
        let app_state = AppState {
            preview: shared_state,
            tx,
        };

        let response = preview_handler_with_ws(AxumState(app_state)).await;
        let html = response.0;

        // Should contain batch-cards container
        assert!(
            html.contains(r#"class="batch-cards""#),
            "HTML should contain batch-cards container"
        );

        // Should contain batch-card div
        assert!(
            html.contains(r#"class="batch-card""#),
            "HTML should contain batch-card div"
        );

        // Should contain batch name
        assert!(
            html.contains("thanksgiving-arrival"),
            "HTML should contain batch name"
        );

        // Should contain file count
        assert!(html.contains("51 files"), "HTML should contain file count");

        // Should contain time range
        assert!(
            html.contains("14:00 - 16:30"),
            "HTML should contain time range"
        );
    }

    #[tokio::test]
    async fn test_preview_handler_shows_thumbnails() {
        // Create state with real thumbnail data
        let input = fixture_path("sample-with-exif.jpg");
        let thumbnail = generate_thumbnail(&input, 100).expect("Failed to generate thumbnail");

        let batches = vec![BatchPreview {
            index: 0,
            name: Some("batch-1".to_string()),
            file_count: 10,
            first_photo: Some("photo1.jpg".to_string()),
            last_photo: Some("photo10.jpg".to_string()),
            first_thumbnail: Some(thumbnail.clone()),
            last_thumbnail: Some(thumbnail.clone()),
            time_range: Some("10:00 - 12:00".to_string()),
        }];

        let state = PreviewState {
            batch_count: 1,
            current_stage: WorkflowStage::Name,
            batches,
        };

        let shared_state = Arc::new(RwLock::new(state));
        let (tx, _rx) = broadcast::channel::<String>(100);
        let app_state = AppState {
            preview: shared_state,
            tx,
        };

        let response = preview_handler_with_ws(AxumState(app_state)).await;
        let html = response.0;

        // Should contain base64 image data URL
        assert!(
            html.contains("data:image/jpeg;base64,"),
            "HTML should contain base64 image data URL"
        );

        // Should contain img tags
        assert!(html.contains("<img"), "HTML should contain img tags");

        // Should have thumbnail class
        assert!(
            html.contains(r#"class="thumbnail"#),
            "HTML should have thumbnail class"
        );
    }
}
