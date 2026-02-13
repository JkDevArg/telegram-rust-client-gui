#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
use app::{TelegramApp, GuiAction, BackendEvent, ChatInfo, MessageInfo};
use grammers_client::{Client, SignInError};
use grammers_mtsender::SenderPool;
use std::sync::Arc;
use tokio::sync::mpsc;
use simple_logger::SimpleLogger;

use grammers_client::types::Peer;

struct BackgroundState {
    api_hash: String,
    login_token: Option<grammers_client::types::LoginToken>,
    password_token: Option<grammers_client::types::PasswordToken>,
    chat_map: std::collections::HashMap<String, Peer>,
}

fn main() -> eframe::Result<()> {
    SimpleLogger::new().with_level(log::LevelFilter::Debug).init().unwrap();
    
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let _enter = rt.enter();

    let (gui_tx, gui_rx) = mpsc::channel(100);
    let (bg_tx, bg_rx) = mpsc::channel(100);

    // Spawn background task
    rt.spawn(async move {
        background_loop(bg_tx, gui_rx).await;
    });

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([400.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Telegram Rust Client",
        options,
        Box::new(|_cc| Ok(Box::new(TelegramApp::new(gui_tx, bg_rx)))),
    )
}

async fn background_loop(tx: mpsc::Sender<BackendEvent>, mut rx: mpsc::Receiver<GuiAction>) {
    // 1. Wait for configuration
    let (api_id, api_hash) = loop {
        match rx.recv().await {
             Some(GuiAction::Configure { api_id, api_hash }) => break (api_id, api_hash),
             Some(_) => {
                 let _ = tx.send(BackendEvent::Error("Please configure API ID first".to_string())).await;
             }
             None => return,
        }
    };

    // 2. Initialize Client
    let session_path = "session.session";
    let session = grammers_session::storages::SqliteSession::open(session_path).unwrap();
    let session = Arc::new(session);
    let pool = SenderPool::new(session.clone(), api_id);
    let client = Client::new(&pool);
    let _pool_handle = pool.handle.clone();
    
    tokio::spawn(async move {
        pool.runner.run().await
    });

    let mut state = BackgroundState {
        api_hash: api_hash.clone(),
        login_token: None,
        password_token: None,
        chat_map: std::collections::HashMap::new(),
    };

    let _ = tx.send(BackendEvent::Configured).await;

    if let Ok(true) = client.is_authorized().await {
        let _ = tx.send(BackendEvent::LoggedIn).await;
    }

    // 3. Main Loop
    while let Some(action) = rx.recv().await {
        match action {
            GuiAction::Login(phone) => {
                 match client.request_login_code(&phone, &state.api_hash).await {
                     Ok(token) => {
                         state.login_token = Some(token);
                         let _ = tx.send(BackendEvent::CodeSent).await;
                     }
                     Err(e) => {
                         let _ = tx.send(BackendEvent::Error(e.to_string())).await;
                     }
                 }
            }
            GuiAction::SendCode(code) => {
                if let Some(token) = &state.login_token {
                    match client.sign_in(token, &code).await {
                        Ok(_) => {
                             let _ = tx.send(BackendEvent::LoggedIn).await;
                        }
                        Err(SignInError::PasswordRequired(ptoken)) => {
                            state.password_token = Some(ptoken);
                            let _ = tx.send(BackendEvent::PasswordRequired).await;
                        }
                         Err(e) => {
                             let _ = tx.send(BackendEvent::Error(e.to_string())).await;
                         }
                    }
                } else {
                    let _ = tx.send(BackendEvent::Error("No login token found".to_string())).await;
                }
            }
            GuiAction::SendPassword(password) => {
                 if let Some(ptoken) = state.password_token.take() {
                     match client.check_password(ptoken, &password).await {
                         Ok(_) => {
                             let _ = tx.send(BackendEvent::LoggedIn).await;
                         }
                         Err(e) => {
                             let _ = tx.send(BackendEvent::Error(e.to_string())).await;
                         }
                     }
                 } else {
                     let _ = tx.send(BackendEvent::Error("No password token found".to_string())).await;
                 }
            }
            GuiAction::RefreshChats => {
                let mut chat_infos = Vec::new();
                let mut dialogs = client.iter_dialogs();
                while let Ok(Some(dialog)) = dialogs.next().await {
                    let chat = dialog.peer();
                    let name = chat.name().unwrap_or("Unknown").to_string();
                    let id = chat.id().to_string();
                    
                    state.chat_map.insert(id.clone(), chat.clone());
                    
                    chat_infos.push(ChatInfo {
                        name,
                        id,
                    });
                    
                    if chat_infos.len() >= 50 { break; }
                }
                let _ = tx.send(BackendEvent::ChatsLoaded(chat_infos)).await;
            }
            GuiAction::SelectChat(chat_id) => {
                if let Some(peer) = state.chat_map.get(&chat_id) {
                    let mut msgs = Vec::new();
                    let mut messages = client.iter_messages(peer).limit(50);
                    while let Ok(Some(message)) = messages.next().await {
                        let sender = message.sender().map(|s| s.name().unwrap_or("Unknown").to_string()).unwrap_or("Unknown".to_string());
                        msgs.push(MessageInfo {
                            id: message.id(),
                            text: message.text().to_string(),
                            sender,
                            date: message.date().to_string(),
                        });
                    }
                    msgs.reverse();
                    let _ = tx.send(BackendEvent::MessagesLoaded(msgs)).await;
                } else {
                    let _ = tx.send(BackendEvent::Error("Chat not found in cache".to_string())).await;
                }
            }
            GuiAction::SendMessage { chat_id, text } => {
                if let Some(peer) = state.chat_map.get(&chat_id) {
                    match client.send_message(peer, text).await {
                        Ok(_) => {
                             // Refresh messages
                            let mut msgs = Vec::new();
                            let mut messages = client.iter_messages(peer).limit(50);
                            while let Ok(Some(message)) = messages.next().await {
                                let sender = message.sender().map(|s| s.name().unwrap_or("Unknown").to_string()).unwrap_or("Unknown".to_string());
                                msgs.push(MessageInfo {
                                    id: message.id(),
                                    text: message.text().to_string(),
                                    sender,
                                    date: message.date().to_string(),
                                });
                            }
                            msgs.reverse();
                            let _ = tx.send(BackendEvent::MessagesLoaded(msgs)).await;
                        }
                        Err(e) => {
                            let _ = tx.send(BackendEvent::Error(format!("Failed to send: {}", e))).await;
                        }
                    }
                }
            }
            GuiAction::Logout => {
                match client.sign_out().await {
                    Ok(_) => {
                        state.login_token = None;
                        state.password_token = None;
                        state.chat_map.clear();
                        let _ = tx.send(BackendEvent::LoggedOut).await;
                    }
                    Err(e) => {
                        let _ = tx.send(BackendEvent::Error(format!("Failed to log out: {}", e))).await;
                    }
                }
            }
            _ => {}
        }
    }
}
