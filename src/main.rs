extern crate redis;
mod style;

use generational_arena::{Arena, Index};
use iced::{
    button, scrollable, text_input, Align, Button, Column, Container, Element, Length, Row,
    Sandbox, Scrollable, Settings, Text, TextInput,
};
use redis::{Commands, Connection};
use std::collections::HashMap;

fn main() -> iced::Result {
    RedisViewer::run(Settings::default())
}

struct RedisViewer {
    server_tabs: Arena<ServerTab>,
    current_server_tab_index: Option<Index>,
    conn_form_state: ConnectionFormState,
    keys_refresh_button_state: button::State,
    tab_buttons: Vec<(String, Index, button::State)>,
    new_tab_button: button::State,
}

struct KeysScrollbarState {
    state: scrollable::State,
}

struct ValueEditState {
    selected_key: String,
    show_value_view: bool,
    selected_value: String,
    selected_value_text_input_state: text_input::State,
    save_button_state: button::State,
}

struct ServerTab {
    name: String,
    redis: redis::Connection,
    keys: Vec<String>,
    //namespaces: HashMap<String, RedisNamespace>,
    keys_scrollbar_state: KeysScrollbarState,
    key_buttons: Vec<(String, button::State)>,
    value_edit_state: ValueEditState,
}

struct ConnectionFormState {
    show_connection_form: bool,
    conn_name_text_input_state: text_input::State,
    conn_name_value: String,
    conn_text_input_state: text_input::State,
    conn_value: String,
    connect_button: button::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    KeySelected(usize),
    SelectedValueChanged(String),
    SelectedValueSaved,
    ConnNameChanged(String),
    ConnValueChanged(String),
    ConnectRedis,
    RefreshKeys,
    ChangeTab(Index),
    NewTab,
}

impl Sandbox for RedisViewer {
    type Message = Message;

    fn new() -> Self {
        let server_tabs = Arena::<ServerTab>::new();

        let current_server_tab_index = None;

        let show_connection_form = true;

        let conn_name_text_input_state = text_input::State::default();
        let conn_name_value = String::from("localhost");
        let conn_text_input_state = text_input::State::default();
        let conn_value = String::from("redis://127.0.0.1");
        let connect_button = button::State::default();

        let conn_form_state = ConnectionFormState {
            show_connection_form,
            conn_name_text_input_state,
            conn_name_value,
            conn_text_input_state,
            conn_value,
            connect_button,
        };

        let refresh_button_state = button::State::default();
        let tab_buttons = Vec::<(String, Index, button::State)>::new();
        let new_tab_button = button::State::default();

        RedisViewer {
            server_tabs,
            current_server_tab_index,
            conn_form_state,
            keys_refresh_button_state: refresh_button_state,
            tab_buttons,
            new_tab_button,
        }
    }

    fn title(&self) -> String {
        String::from("Icy Redis Viewer")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::KeySelected(i) => {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");
                let key = current_server_tab.keys[i].clone();
                current_server_tab.value_edit_state.selected_key = key.clone();
                current_server_tab.value_edit_state.selected_value =
                    get_redis_value(&mut current_server_tab.redis, key)
                        .expect("failed to get value for selected redis key");
                current_server_tab.value_edit_state.show_value_view = true;
            }
            Message::SelectedValueChanged(s) => {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");
                current_server_tab.value_edit_state.selected_value = s;
            }
            Message::SelectedValueSaved => {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");
                set_redis_value(
                    &mut current_server_tab.redis,
                    current_server_tab.value_edit_state.selected_key.clone(),
                    current_server_tab.value_edit_state.selected_value.clone(),
                )
                .expect("failed to set redis value");
            }
            Message::ConnNameChanged(s) => {
                self.conn_form_state.conn_name_value = s;
            }
            Message::ConnValueChanged(s) => {
                self.conn_form_state.conn_value = s;
            }
            Message::ConnectRedis => {
                let conn = self.conn_form_state.conn_value.clone();
                let mut redis = connect_redis(&conn).expect("failed to get redis connection");
                let keys = get_all_keys(&mut redis).expect("failed to get keys");
                //let namespaces = convert_keys_to_namespaces(&keys);
                let keys_scrollbar_state = KeysScrollbarState {
                    state: scrollable::State::new(),
                };
                let mut key_buttons = Vec::<(String, button::State)>::new();
                for key in keys.iter() {
                    key_buttons.push((key.clone(), button::State::default()));
                }

                let selected_key = String::new();
                let show_value_view = false;
                let selected_value = String::new();
                let selected_value_text_input_state = text_input::State::default();

                let save_button_state = button::State::default();

                let value_edit_state = ValueEditState {
                    selected_key,
                    show_value_view,
                    selected_value,
                    selected_value_text_input_state,
                    save_button_state,
                };

                let name = self.conn_form_state.conn_name_value.clone();

                let server_tab = ServerTab {
                    name,
                    redis,
                    keys,
                    keys_scrollbar_state,
                    //namespaces,
                    key_buttons,
                    value_edit_state,
                };
                self.current_server_tab_index = Some(self.server_tabs.insert(server_tab));
                self.tab_buttons.push((
                    self.conn_form_state.conn_name_value.clone(),
                    self.current_server_tab_index
                        .expect("failed to get current server index"),
                    button::State::default(),
                ));
                self.conn_form_state.show_connection_form = false;
            }
            Message::RefreshKeys => {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");
                let keys = get_all_keys(&mut current_server_tab.redis).expect("failed to get keys");
                //let namespaces = convert_keys_to_namespaces(&keys);
                let keys_scrollbar_state = KeysScrollbarState {
                    state: scrollable::State::new(),
                };
                let mut key_buttons = Vec::<(String, button::State)>::new();
                for key in keys.iter() {
                    key_buttons.push((key.clone(), button::State::default()));
                }

                current_server_tab.keys = keys;
                //current_server_tab.namespaces = namespaces;
                current_server_tab.keys_scrollbar_state = keys_scrollbar_state;
                current_server_tab.key_buttons = key_buttons;
            }
            Message::ChangeTab(i) => {
                self.current_server_tab_index = Some(i);
            }
            Message::NewTab => {
                self.conn_form_state.show_connection_form = true;
                self.current_server_tab_index = None;
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        let content =
            if self.conn_form_state.show_connection_form || self.current_server_tab_index == None {
                let connection_form = Column::new()
                    .push(
                        Row::new()
                            .padding(10)
                            .push(
                                TextInput::new(
                                    &mut self.conn_form_state.conn_name_text_input_state,
                                    "Enter the nickname for your redis server here.",
                                    &self.conn_form_state.conn_name_value,
                                    Message::ConnNameChanged,
                                )
                                .padding(5),
                            )
                            .push(
                                TextInput::new(
                                    &mut self.conn_form_state.conn_text_input_state,
                                    "Enter the url for your redis server here.",
                                    &self.conn_form_state.conn_value,
                                    Message::ConnValueChanged,
                                )
                                .padding(5),
                            ),
                    )
                    .push(
                        Row::new().padding(10).push(
                            Button::new(
                                &mut self.conn_form_state.connect_button,
                                Text::new("Connect"),
                            )
                            .on_press(Message::ConnectRedis),
                        ),
                    );

                Column::new()
                    .align_items(Align::Center)
                    .spacing(20)
                    .push(connection_form)
            } else {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");

                let tabs = self
                    .tab_buttons
                    .iter_mut()
                    .enumerate()
                    .fold(
                        Row::new()
                            .align_items(Align::Start)
                            .width(Length::Fill)
                            .height(Length::Shrink),
                        |row, (_i, (name, index, state))| {
                            row.push(
                                Button::new(state, Text::new(name.clone()))
                                    .on_press(Message::ChangeTab(*index)),
                            )
                        },
                    )
                    .push(
                        Button::new(&mut self.new_tab_button, Text::new("New"))
                            .on_press(Message::NewTab),
                    );

                let keys = current_server_tab.key_buttons.iter_mut().enumerate().fold(
                    Scrollable::new(&mut current_server_tab.keys_scrollbar_state.state)
                        .padding(0)
                        .align_items(Align::Start)
                        .width(Length::Fill)
                        .height(Length::Fill),
                    |scrollable, (i, (key, state))| {
                        scrollable.push(
                            Row::new().push(
                                Button::new(state, Text::new(key.clone()))
                                    .padding(5)
                                    .on_press(Message::KeySelected(i)),
                            ),
                        )
                    },
                );

                let value_column = if current_server_tab.value_edit_state.show_value_view {
                    Column::new()
                        .align_items(Align::Start)
                        .width(Length::FillPortion(3))
                        .height(Length::Fill)
                        .push(
                            Row::new()
                                .padding(20)
                                .push(Text::new(&current_server_tab.value_edit_state.selected_key)),
                        )
                        .push(
                            Row::new().padding(20).push(
                                TextInput::new(
                                    &mut current_server_tab
                                        .value_edit_state
                                        .selected_value_text_input_state,
                                    "Enter your redis value here.",
                                    &current_server_tab.value_edit_state.selected_value,
                                    Message::SelectedValueChanged,
                                )
                                .width(Length::Fill)
                                .padding(10),
                            ),
                        )
                        .push(
                            Row::new().padding(20).push(
                                Button::new(
                                    &mut current_server_tab.value_edit_state.save_button_state,
                                    Text::new("Save"),
                                )
                                .on_press(Message::SelectedValueSaved),
                            ),
                        )
                } else {
                    Column::new()
                        .align_items(Align::Start)
                        .width(Length::FillPortion(3))
                        .height(Length::Fill)
                        .padding(20)
                };

                let tab_controls = Row::new()
                    .width(Length::Fill)
                    .height(Length::Shrink)
                    .push(
                        Column::new()
                            .padding(10)
                            .push(Text::new(&current_server_tab.name)),
                    )
                    .push(
                        Column::new().padding(5).push(
                            Button::new(&mut self.keys_refresh_button_state, Text::new("Refresh"))
                                .on_press(Message::RefreshKeys),
                        ),
                    );

                let viewer_row = Row::new()
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(10)
                    .push(keys)
                    .push(value_column);

                Column::new()
                    .align_items(Align::Center)
                    .spacing(20)
                    .push(tabs)
                    .push(tab_controls)
                    .push(viewer_row)
            };

        let theme = style::Theme::Dark;

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(theme)
            .into()
    }

    fn background_color(&self) -> iced::Color {
        iced::Color::WHITE
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }
}

fn connect_redis(connection: &str) -> redis::RedisResult<Connection> {
    let client = redis::Client::open(connection)?;
    client.get_connection()
}

fn get_all_keys(redis: &mut redis::Connection) -> redis::RedisResult<Vec<String>> {
    let all_keys: Vec<String> = redis.keys("*")?;
    Ok(all_keys)
}

fn get_redis_value(redis: &mut redis::Connection, key: String) -> redis::RedisResult<String> {
    let value: String = redis.get(key)?;
    Ok(value)
}

fn set_redis_value(
    con: &mut redis::Connection,
    key: String,
    value: String,
) -> redis::RedisResult<()> {
    let _: () = con.set(key, value)?;
    Ok(())
}

fn convert_keys_to_namespaces(keys: &Vec<String>) -> HashMap<String, RedisNamespace> {
    let mut namespaces = HashMap::<String, RedisNamespace>::new();

    let mut empty_namespace = RedisNamespace {
        name: "".into(),
        sub_namespaces: HashMap::<String, RedisNamespace>::new(),
        keys: Vec::<String>::new(),
    };

    for key in keys {
        let parts: Vec<&str> = key.split(":").collect();
        if parts.len() == 1 {
            empty_namespace.keys.push(key.clone());
        } else {
            add_key_to_namespaces(parts, &mut namespaces, 0);
        }
    }
    namespaces.insert("".into(), empty_namespace);
    namespaces
}

fn add_key_to_namespaces(
    parts: Vec<&str>,
    current_namespace: &mut HashMap<String, RedisNamespace>,
    part_index: usize,
) {
    let part = parts[part_index];
    let result = current_namespace.get_mut(part);

    let next_namespace = match result {
        Some(namespace) => namespace,
        None => {
            current_namespace.insert(
                part.into(),
                RedisNamespace {
                    name: part.into(),
                    sub_namespaces: HashMap::<String, RedisNamespace>::new(),
                    keys: Vec::<String>::new(),
                },
            );
            current_namespace.get_mut(part).unwrap()
        }
    };

    if part_index == parts.len() - 1 {
        next_namespace.keys.push(parts.join(":"));
    } else {
        add_key_to_namespaces(parts, &mut next_namespace.sub_namespaces, part_index + 1);
    }
}

struct RedisNamespace {
    name: String,
    sub_namespaces: HashMap<String, RedisNamespace>,
    keys: Vec<String>,
}
