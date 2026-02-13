use eframe::egui;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum GuiState {
    Configuration,
    LoginPhone,
    LoginCode,
    LoginPassword,
    LoggedIn,
}

pub enum GuiAction {
    Configure { api_id: i32, api_hash: String },
    Login(String),
    SendCode(String),
    SendPassword(String),
    RefreshChats,
    SelectChat(String),
    SendMessage { chat_id: String, text: String },
    Logout,
    BackToChats,
}

#[derive(Debug, Clone)]
pub struct ChatInfo {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct MessageInfo {
    pub id: i32,
    pub text: String,
    pub sender: String,
    pub date: String,
}

#[derive(Debug)]
pub enum BackendEvent {
    Configured,
    CodeSent,
    PasswordRequired,
    LoggedIn,
    ChatsLoaded(Vec<ChatInfo>),
    MessagesLoaded(Vec<MessageInfo>),
    LoggedOut,
    Error(String),
}

pub struct TelegramApp {
    state: GuiState,
    api_id_input: String,
    api_hash_input: String,
    phone: String,
    code: String,
    password: String,
    chats: Vec<ChatInfo>,
    messages: Vec<MessageInfo>,
    selected_chat: Option<ChatInfo>,
    message_input: String,
    tx: mpsc::Sender<GuiAction>,
    rx: mpsc::Receiver<BackendEvent>,
    status_message: String,
}

impl TelegramApp {
    pub fn new(tx: mpsc::Sender<GuiAction>, rx: mpsc::Receiver<BackendEvent>) -> Self {
        Self {
            state: GuiState::Configuration,
            api_id_input: "".to_string(),
            api_hash_input: "".to_string(),
            phone: "".to_string(),
            code: String::new(),
            password: String::new(),
            chats: Vec::new(),
            messages: Vec::new(),
            selected_chat: None,
            message_input: String::new(),
            tx,
            rx,
            status_message: "Please enter API ID and Hash".to_string(),
        }
    }

    fn handle_backend_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                BackendEvent::Configured => {
                    self.state = GuiState::LoginPhone;
                    self.status_message = "Configuration set. Enter phone number.".to_string();
                }
                BackendEvent::CodeSent => {
                    self.state = GuiState::LoginCode;
                    self.status_message = "Code sent! Check Telegram.".to_string();
                }
                BackendEvent::PasswordRequired => {
                    self.state = GuiState::LoginPassword;
                    self.status_message = "2FA Password Required.".to_string();
                }
                BackendEvent::LoggedIn => {
                    self.state = GuiState::LoggedIn;
                    self.status_message = "Logged in successfully!".to_string();
                    let _ = self.tx.try_send(GuiAction::RefreshChats);
                }
                BackendEvent::ChatsLoaded(chats) => {
                    self.chats = chats;
                    self.status_message = "Chats loaded.".to_string();
                }
                BackendEvent::MessagesLoaded(msgs) => {
                    self.messages = msgs;
                    self.status_message = "Messages loaded.".to_string();
                }
                BackendEvent::LoggedOut => {
                    self.state = GuiState::LoginPhone;
                    self.chats.clear();
                    self.messages.clear();
                    self.selected_chat = None;
                    self.status_message = "Logged out.".to_string();
                }
                BackendEvent::Error(msg) => {
                    self.status_message = format!("Error: {}", msg);
                }
            }
        }
    }
}

impl eframe::App for TelegramApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_backend_events();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Telegram Rust Client");
            ui.label(&self.status_message);
            ui.separator();

            match self.state {
                GuiState::Configuration => {
                    ui.horizontal(|ui| {
                        ui.label("API ID:");
                        ui.text_edit_singleline(&mut self.api_id_input);
                    });
                    ui.horizontal(|ui| {
                        ui.label("API Hash:");
                        ui.text_edit_singleline(&mut self.api_hash_input);
                    });
                    if ui.button("Save Configuration").clicked() {
                        if let Ok(api_id) = self.api_id_input.parse::<i32>() {
                             let _ = self.tx.try_send(GuiAction::Configure {
                                 api_id,
                                 api_hash: self.api_hash_input.clone(),
                             });
                             self.status_message = "Saving configuration...".to_string();
                        } else {
                            self.status_message = "Invalid API ID (must be a number)".to_string();
                        }
                    }
                }
                GuiState::LoginPhone => {
                    ui.horizontal(|ui| {
                        ui.label("Phone Number:");
                        ui.text_edit_singleline(&mut self.phone);
                    });
                    if ui.button("Send Code").clicked() {
                        self.status_message = "Sending code...".to_string();
                        let _ = self.tx.try_send(GuiAction::Login(self.phone.clone()));
                    }
                }
                GuiState::LoginCode => {
                    ui.horizontal(|ui| {
                        ui.label("Login Code:");
                        ui.text_edit_singleline(&mut self.code);
                    });
                    if ui.button("Sign In").clicked() {
                        self.status_message = "Verifying code...".to_string();
                        let _ = self.tx.try_send(GuiAction::SendCode(self.code.clone()));
                    }
                }
                GuiState::LoginPassword => {
                    ui.horizontal(|ui| {
                        ui.label("2FA Password:");
                        ui.text_edit_singleline(&mut self.password);
                    });
                    if ui.button("Verify Password").clicked() {
                        self.status_message = "Verifying password...".to_string();
                        let _ = self.tx.try_send(GuiAction::SendPassword(self.password.clone()));
                    }
                }
                GuiState::LoggedIn => {
                    if let Some(selected_chat) = self.selected_chat.clone() {
                         ui.horizontal(|ui| {
                             if ui.button("Back").clicked() {
                                 self.selected_chat = None;
                                 self.messages.clear();
                                 let _ = self.tx.try_send(GuiAction::BackToChats);
                             }
                             ui.label(format!("Chat: {}", selected_chat.name));
                         });
                         ui.separator();
                         
                         // Messages Area
                         egui::ScrollArea::vertical()
                             .max_height(ui.available_height() - 50.0)
                             .show(ui, |ui| {
                             for msg in &self.messages {
                                 ui.group(|ui| {
                                     ui.horizontal(|ui| {
                                         ui.strong(&msg.sender);
                                         ui.weak(&msg.date);
                                     });
                                     ui.label(&msg.text);
                                 });
                             }
                         });
                         
                         ui.separator();
                         
                         // Input Area
                         ui.horizontal(|ui| {
                             ui.text_edit_singleline(&mut self.message_input);
                             if ui.button("Send").clicked() {
                                 let text = self.message_input.clone();
                                 if !text.is_empty() {
                                     let _ = self.tx.try_send(GuiAction::SendMessage {
                                         chat_id: selected_chat.id.clone(),
                                         text,
                                     });
                                     self.message_input.clear();
                                     self.status_message = "Sending message...".to_string();
                                 }
                             }
                         });
                    } else {
                        ui.horizontal(|ui| {
                            if ui.button("Refresh Chats").clicked() {
                                let _ = self.tx.try_send(GuiAction::RefreshChats);
                            }
                            if ui.button("Logout").clicked() {
                                let _ = self.tx.try_send(GuiAction::Logout);
                            }
                        });
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for chat in &self.chats {
                                if ui.button(&chat.name).clicked() {
                                    self.selected_chat = Some(chat.clone());
                                    self.status_message = format!("Loading messages for {}...", chat.name);
                                    let _ = self.tx.try_send(GuiAction::SelectChat(chat.id.clone()));
                                }
                            }
                        });
                    }
                }
            }
        });
    }
}
