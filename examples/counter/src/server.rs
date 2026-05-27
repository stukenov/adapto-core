use adapto_compiler::compiler::Compiler;
use adapto_compiler::ir::ComponentIR;
use adapto_live::manager::SessionManager;
use adapto_live::session::LiveSession;
use adapto_runtime::context::PermissionSet;
use adapto_runtime::state::StateStore;
use adapto_runtime::types::*;
use adapto_ssr::renderer::Renderer;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;

const COUNTER_DSL: &str = r#"
<route>
  path: "/counter"
  auth: public
</route>

<script lang="rust">
  state count: i32 = 0

  action fn increment() {
    count += 1
  }

  action fn decrement() {
    count -= 1
  }

  action fn reset() {
    count = 0
  }
</script>

<template>
  <div>
    <h1>Adapto Counter</h1>
    <p>Count: {count}</p>
    <button on:click="increment">+1</button>
    <button on:click="decrement">-1</button>
    <button on:click="reset">Reset</button>
  </div>
</template>

<style scoped>
  div { max-width: 400px; margin: 4rem auto; text-align: center; font-family: -apple-system, system-ui, sans-serif; }
  h1 { font-size: 2rem; margin-bottom: 0.5rem; }
  p { font-size: 3rem; font-weight: bold; margin: 1rem 0; }
  button { font-size: 1.2rem; padding: 0.6rem 1.5rem; margin: 0.25rem; border: 1px solid #ccc; border-radius: 8px; cursor: pointer; background: #f5f5f7; }
  button:hover { background: #007aff; color: white; border-color: #007aff; }
</style>
"#;

struct App {
    ir: ComponentIR,
    dep_graph: adapto_compiler::dependency::DependencyGraph,
    renderer: Renderer,
    session_manager: SessionManager,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let ast = adapto_parser::parse(COUNTER_DSL).expect("Parse failed");
    let mut compiler = Compiler::new();
    let output = compiler
        .compile_file(&ast, "counter.adapto")
        .expect("Compile failed");

    let app = Arc::new(App {
        ir: output.component_ir,
        dep_graph: output.dependency_graph,
        renderer: Renderer::new(b"counter-secret-key"),
        session_manager: SessionManager::new(100),
    });

    let router = Router::new()
        .route("/", get(handle_page))
        .route("/ws", get(handle_ws))
        .with_state(app);

    let addr = "127.0.0.1:3000";
    println!();
    println!("  Adapto Counter running at http://{addr}");
    println!("  Press Ctrl+C to stop.");
    println!();

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

async fn handle_page(State(app): State<Arc<App>>) -> impl IntoResponse {
    let mut state = StateStore::new();
    state.set("count", json!(0));
    state.clear_dirty();

    let (html, _session_id) = app
        .renderer
        .render_page(&app.ir, &state, None)
        .unwrap_or_else(|e| (format!("<h1>Error: {e}</h1>"), String::new()));

    let html = html.replace(
        "<script src=\"/assets/adapto-client.js\"></script>",
        &inline_client_script(),
    );

    Html(html)
}

async fn handle_ws(ws: WebSocketUpgrade, State(app): State<Arc<App>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_loop(socket, app))
}

async fn ws_loop(mut socket: WebSocket, app: Arc<App>) {
    let session_id = SessionId::from("live-counter-1");

    // Create session and init state from IR defaults (no manual handlers needed)
    let mut session = LiveSession::new(
        session_id.clone(),
        None,
        None,
        RouteId::from("/counter"),
        app.ir.clone(),
        app.dep_graph.clone(),
        PermissionSet::new(),
    );
    session.init_state_from_defaults();
    let _ = app.session_manager.add(session);

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(val) => {
                    let handler = val
                        .get("handler")
                        .and_then(|h| h.as_str())
                        .unwrap_or("");

                    let response = app.session_manager.with_session(&session_id, |s| {
                        let event = adapto_client_protocol::event::ClientEvent {
                            session: session_id.0.clone(),
                            component: "counter".into(),
                            event: "click".into(),
                            handler: handler.to_string(),
                            payload: std::collections::HashMap::new(),
                            seq: val.get("seq").and_then(|s| s.as_u64()).unwrap_or(0),
                        };

                        match s.handle_event(&event) {
                            Ok(patch) => {
                                let server_msg =
                                    adapto_client_protocol::patch::ServerMessage::new(
                                        adapto_client_protocol::patch::ServerPayload::Patch(patch),
                                    );
                                serde_json::to_string(&server_msg).ok()
                            }
                            Err(e) => {
                                let err_msg =
                                    adapto_client_protocol::patch::ServerMessage::new(
                                        adapto_client_protocol::patch::ServerPayload::Error(
                                            adapto_client_protocol::patch::ErrorMessage {
                                                seq: None,
                                                code: "HANDLER_ERROR".into(),
                                                message: e.to_string(),
                                            },
                                        ),
                                    );
                                serde_json::to_string(&err_msg).ok()
                            }
                        }
                    });

                    if let Ok(Some(json_str)) = response {
                        let _ = socket.send(Message::Text(json_str.into())).await;
                    }
                }
                Err(e) => {
                    eprintln!("Bad message: {e}");
                }
            }
        }
    }

    app.session_manager.remove(&session_id);
}

fn inline_client_script() -> String {
    r#"<script>
(function() {
  var ws, seq = 0;
  var proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  var url = proto + '//' + location.host + '/ws';

  function connect() {
    ws = new WebSocket(url);
    ws.onopen = function() { console.log('[Adapto] Connected'); };
    ws.onmessage = function(e) {
      try {
        var msg = JSON.parse(e.data);
        if (msg.type === 'patch' && msg.ops) {
          msg.ops.forEach(function(op) {
            if (op.op === 'replace_text') {
              var el = document.querySelector('[data-ar-dyn="' + op.target + '"]');
              if (el) el.textContent = op.value;
            }
          });
        }
      } catch(err) { console.error(err); }
    };
    ws.onclose = function() { setTimeout(connect, 2000); };
  }

  document.addEventListener('click', function(e) {
    var btn = e.target.closest('button');
    if (!btn) return;
    var text = btn.textContent.trim();
    var handler = '';
    if (text === '+1') handler = 'increment';
    else if (text === '-1') handler = 'decrement';
    else if (text === 'Reset') handler = 'reset';
    if (handler && ws && ws.readyState === 1) {
      ws.send(JSON.stringify({ v:1, type:'event', session:'live-counter-1', component:'counter', event:'click', handler:handler, payload:{}, seq:++seq }));
    }
  });

  connect();
})();
</script>"#.to_string()
}
