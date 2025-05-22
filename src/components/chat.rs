// chat.rs — versi lengkap setelah modifikasi fitur "typing indicator" (bubble "...")

use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;
// use yew::events::InputData; // Removed: InputData is deprecated
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

use crate::services::event_bus::EventBus;
use crate::{services::websocket::WebsocketService, User};

// =====================
// Messages (Component <-> Runtime)
// =====================
#[derive(Debug)]
pub enum Msg {
    /// Pesan yang diteruskan dari EventBus / WebSocket
    HandleMsg(String),
    /// Klik tombol kirim
    SubmitMessage,
    /// Perubahan pada input chat (mengetik)
    TypingChanged(String),
}

// =====================
// Tipe data dari server WebSocket
// =====================
#[derive(Deserialize)]
struct MessageData {
    from: String,
    message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MsgTypes {
    Users,
    Register,
    Message,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebSocketMessage {
    message_type: MsgTypes,
    data_array: Option<Vec<String>>,
    data: Option<String>,
}

// =====================
// Profil user (sidebar)
// =====================
#[derive(Clone)]
struct UserProfile {
    name: String,
    avatar: String,
}

// =====================
// Komponen Chat
// =====================
pub struct Chat {
    // --- UI state
    users: Vec<UserProfile>,
    messages: Vec<MessageData>,
    is_typing: bool,

    // --- Refs & services
    chat_input: NodeRef,
    _producer: Box<dyn Bridge<EventBus>>, // listener EventBus
    wss: WebsocketService,                // WebSocket service
}

impl Component for Chat {
    type Message = Msg;
    type Properties = ();

    // ---------- create ----------
    fn create(ctx: &Context<Self>) -> Self {
        // Ambil username dari context
        let (user, _) = ctx
            .link()
            .context::<User>(Callback::noop())
            .expect("context to be set");

        let wss = WebsocketService::new();
        let username = user.username.borrow().clone();

        // Kirim pesan register ke server
        let register_msg = WebSocketMessage {
            message_type: MsgTypes::Register,
            data: Some(username.clone()),
            data_array: None,
        };
        let _ = wss
            .tx
            .clone()
            .try_send(serde_json::to_string(&register_msg).unwrap());

        Self {
            users: vec![],
            messages: vec![],
            is_typing: false,
            chat_input: NodeRef::default(),
            wss,
            _producer: EventBus::bridge(ctx.link().callback(Msg::HandleMsg)),
        }
    }

    // ---------- update ----------
    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            // ------------
            // EventBus / WebSocket message
            // ------------
            Msg::HandleMsg(raw) => {
                let ws_msg: WebSocketMessage = serde_json::from_str(&raw).unwrap();
                match ws_msg.message_type {
                    MsgTypes::Users => {
                        // Perbarui daftar user
                        let users_from_message = ws_msg.data_array.unwrap_or_default();
                        self.users = users_from_message
                            .iter()
                            .map(|u| UserProfile {
                                name: u.into(),
                                avatar: format!(
                                    "https://avatars.dicebear.com/api/adventurer-neutral/{}.svg",
                                    u
                                ),
                            })
                            .collect();
                        true
                    }
                    MsgTypes::Message => {
                        // Tambahkan pesan baru
                        if let Some(data) = ws_msg.data {
                            if let Ok(message_data) = serde_json::from_str::<MessageData>(&data) {
                                self.messages.push(message_data);
                            }
                        }
                        true
                    }
                    _ => false,
                }
            }
            // ------------
            // Tombol kirim ditekan
            // ------------
            Msg::SubmitMessage => {
                if let Some(input) = self.chat_input.cast::<HtmlInputElement>() {
                    let value = input.value();
                    if value.trim().is_empty() {
                        return false; // abaikan pesan kosong
                    }

                    // Kirim ke WebSocket
                    let message = WebSocketMessage {
                        message_type: MsgTypes::Message,
                        data: Some(value),
                        data_array: None,
                    };
                    let _ = self
                        .wss
                        .tx
                        .clone()
                        .try_send(serde_json::to_string(&message).unwrap());

                    // Kosongkan input & reset indikator
                    input.set_value("");
                    self.is_typing = false;
                }
                true
            }
            // ------------
            // Perubahan teks input (typing)
            // ------------
            Msg::TypingChanged(val) => {
                let currently_typing = !val.trim().is_empty();
                if self.is_typing != currently_typing {
                    self.is_typing = currently_typing;
                    true // rerender hanya jika status berubah
                } else {
                    false
                }
            }
        }
    }

    // ---------- view ----------
    fn view(&self, ctx: &Context<Self>) -> Html {
        // Callbacks
        let submit = ctx.link().callback(|_| Msg::SubmitMessage);
        let oninput = ctx
            .link()
            .callback(|e: InputEvent| {
                let input: HtmlInputElement = e.target_unchecked_into();
                Msg::TypingChanged(input.value())
            });

        html! {
            <div class="flex w-screen">
                // ================= Sidebar Users =================
                <div class="flex-none w-56 h-screen bg-gray-100">
                    <div class="text-xl p-3">{"Users"}</div>
                    { for self.users.iter().map(|u| html!{
                        <div class="flex m-3 bg-white rounded-lg p-2">
                            <img class="w-12 h-12 rounded-full" src={u.avatar.clone()} alt="avatar"/>
                            <div class="flex-grow p-3">
                                <div class="flex text-xs justify-between">
                                    <div>{u.name.clone()}</div>
                                </div>
                                <div class="text-xs text-gray-400">{"Hi there!"}</div>
                            </div>
                        </div>
                    }) }
                </div>

                // ================= Main Chat Pane =================
                <div class="grow h-screen flex flex-col">
                    // ----- Header -----
                    <div class="w-full h-14 border-b-2 border-gray-300">
                        <div class="text-xl p-3">{"💬 Chat!"}</div>
                    </div>

                    // ----- Messages list -----
                    <div class="w-full grow overflow-auto border-b-2 border-gray-300">
                        // Bubble "..." – tampil saat mengetik
                        {
                            if self.is_typing {
                                html! {<div class="flex items-end w-3/6 bg-gray-200 m-8 rounded-tl-lg rounded-tr-lg rounded-br-lg p-3 italic text-gray-600">{"..."}</div>}
                            } else {
                                Html::default()
                            }
                        }

                        { for self.messages.iter().map(|m| {
                            let user = self.users.iter().find(|u| u.name == m.from)
                                .cloned()
                                .unwrap_or(UserProfile { name: m.from.clone(), avatar: String::new() });
                            html!{
                                <div class="flex items-end w-3/6 bg-gray-100 m-8 rounded-tl-lg rounded-tr-lg rounded-br-lg ">
                                    <img class="w-8 h-8 rounded-full m-3" src={user.avatar.clone()} alt="avatar"/>
                                    <div class="p-3">
                                        <div class="text-sm">{m.from.clone()}</div>
                                        <div class="text-xs text-gray-500">
                                            {
                                                if m.message.ends_with(".gif") {
                                                    html!{<img class="mt-3" src={m.message.clone()} />}
                                                } else {
                                                    html!{m.message.clone()}
                                                }
                                            }
                                        </div>
                                    </div>
                                </div>
                            }
                        }) }
                    </div>

                    // ----- Input bar -----
                    <div class="w-full h-14 flex px-3 items-center">
                        <input
                            ref={self.chat_input.clone()}
                            type="text"
                            placeholder="Message"
                            class="block w-full py-2 pl-4 mx-3 bg-gray-100 rounded-full outline-none focus:text-gray-700"
                            name="message"
                            required=true
                            {oninput}
                        />
                        <button onclick={submit} class="p-3 shadow-sm bg-blue-600 w-10 h-10 rounded-full flex justify-center items-center color-white">
                            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" class="fill-white">
                                <path d="M0 0h24v24H0z" fill="none"></path>
                                <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"></path>
                            </svg>
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
