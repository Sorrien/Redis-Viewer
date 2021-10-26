#![feature(default_free_fn)]

extern crate redis;
mod redislogic;
mod style;

use std::{collections::HashMap, default::default};

use crate::redislogic::redislogic::get_redis_value;
use generational_arena::{Arena, Index};
use iced::{
    button, scrollable, text_input, Align, Button, Column, Container, Element, Length, Row,
    Sandbox, Scrollable, Text, TextInput,
};
use redislogic::redislogic::{
    connect_redis, convert_keys_to_namespaces, delete_redis_key, get_all_keys, set_redis_value,
    RedisNamespace, RedisValue,
};

pub struct RedisViewer {
    server_tabs: Arena<ServerTab>,
    current_server_tab_index: Option<Index>,
    conn_form_state: ConnectionFormState,
    keys_refresh_button_state: button::State,
    tab_buttons: Vec<(String, Index, button::State)>,
    new_tab_button: button::State,
    create_key_button: button::State,
}

#[derive(Default)]
struct KeysScrollbarState {
    state: scrollable::State,
}

struct ServerTab {
    name: String,
    redis: redis::Connection,
    keys: Vec<String>,
    namespaces: HashMap<String, RedisNamespace>,
    namespaces_view: Vec<NamespaceView>,
    keys_scrollbar_state: KeysScrollbarState,
    key_buttons: Vec<(String, button::State)>,
    editor_state: EditorState,
}

struct ConnectionFormState {
    show_connection_form: bool,
    conn_name_text_input_state: text_input::State,
    conn_name_value: String,
    conn_text_input_state: text_input::State,
    conn_value: String,
    port_text_input_state: text_input::State,
    port_value: String,
    db_text_input_state: text_input::State,
    db_value: String,
    connect_button: button::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    KeySelected(String),
    SelectedValueChanged(String),
    SelectedValueSaved,
    SelectedValueDeleted,
    ConnNameChanged(String),
    ConnValueChanged(String),
    PortValueChanged(String),
    DbValueChanged(String),
    ConnectRedis,
    RefreshKeys,
    ChangeTab(Index),
    NewTab,
    OpenCreateKeyForm,
    CreateKey,
    CreateKeyChanged(String),
    CreateValueChanged(String),
    NamespaceExpandToggle(Vec<usize>),
}

#[derive(Debug, Clone)]
enum EditorState {
    Empty,
    Edit(ValueEditState),
    Create(KeyCreateState),
}

#[derive(Debug, Clone, Default)]
struct ValueEditState {
    key: String,
    value: String,
    value_input_state: text_input::State,
    save_button_state: button::State,
    delete_button_state: button::State,
}

#[derive(Debug, Clone, Default)]
struct KeyCreateState {
    key: String,
    key_input_state: text_input::State,
    value: String,
    value_input_state: text_input::State,
    create_button_state: button::State,
}

struct NamespaceView {
    namespace: String,
    expand_button_state: button::State,
    is_expanded: bool,
    sub_namespaces: Vec<NamespaceView>,
    key_buttons: Vec<(String, button::State)>,
}

impl NamespaceView {
    fn new(redis_namespace: &RedisNamespace) -> Self {
        let mut key_buttons = Vec::<(String, button::State)>::new();
        for key in redis_namespace.keys.iter() {
            key_buttons.push((key.clone(), button::State::default()));
        }

        let sub_namespaces = redis_namespace
            .sub_namespaces
            .iter()
            .map(|(_, ns)| NamespaceView::new(ns))
            .collect();

        NamespaceView {
            namespace: redis_namespace.name.clone(),
            expand_button_state: button::State::default(),
            is_expanded: false,
            sub_namespaces,
            key_buttons,
        }
    }
}

impl RedisViewer {
    fn refresh_keys(&mut self) {
        let current_server_tab = self
            .server_tabs
            .get_mut(
                self.current_server_tab_index
                    .expect("failed to find current server tab index"),
            )
            .expect("failed to find current server tab in arena");
        let keys = get_all_keys(&mut current_server_tab.redis).expect("failed to get keys");
        let namespaces = convert_keys_to_namespaces(&keys);
        let namespaces_view = create_namespace_views(&namespaces);
        let mut key_buttons = Vec::<(String, button::State)>::new();
        for key in keys.iter() {
            key_buttons.push((key.clone(), button::State::default()));
        }

        current_server_tab.keys = keys;
        current_server_tab.namespaces = namespaces;
        current_server_tab.keys_scrollbar_state = KeysScrollbarState::default();
        current_server_tab.key_buttons = key_buttons;
        current_server_tab.namespaces_view = namespaces_view;
    }
}

fn create_namespace_views(namespaces: &HashMap<String, RedisNamespace>) -> Vec<NamespaceView> {
    let namespaces_view = namespaces
        .iter()
        .map(|(_, ns)| NamespaceView::new(ns))
        .collect();

    namespaces_view
}

fn create_namespace_rows(namespace: &mut NamespaceView, indices: Vec<usize>) -> Row<Message> {
    let expander_text = if namespace.is_expanded {
        Text::new("^")
    } else {
        Text::new(">")
    };

    let ns_row = Row::new()
        .push(
            Button::new(&mut namespace.expand_button_state, expander_text)
                .padding(5)
                .on_press(Message::NamespaceExpandToggle(indices.clone())),
        )
        .push(Text::new(namespace.namespace.clone()));

    let column = Column::new().padding(indices.len() as u16 * 2).push(ns_row);

    let column = if namespace.is_expanded {
        column
            .push(namespace.sub_namespaces.iter_mut().enumerate().fold(
                Column::new(),
                |col, (i, sub_ns)| {
                    let mut current_indices = indices.clone();
                    current_indices.push(i);
                    col.push(create_namespace_rows(sub_ns, current_indices))
                },
            ))
            .push(Row::new().push(namespace.key_buttons.iter_mut().fold(
                Column::new(),
                |col, (key, state)| {
                    col.push(
                        Row::new().push(
                            Button::new(state, Text::new(key.clone()))
                                .padding(5)
                                .on_press(Message::KeySelected(key.clone())),
                        ),
                    )
                },
            )))
    } else {
        column
    };

    Row::new().push(column)
}

impl Sandbox for RedisViewer {
    type Message = Message;

    fn new() -> Self {
        let server_tabs = Arena::<ServerTab>::new();

        let current_server_tab_index = None;

        let conn_form_state = ConnectionFormState {
            show_connection_form: true,
            conn_name_text_input_state: text_input::State::default(),
            conn_name_value: String::from("localhost"),
            conn_text_input_state: text_input::State::default(),
            conn_value: String::from("127.0.0.1"),
            connect_button: button::State::default(),
            port_text_input_state: text_input::State::default(),
            port_value: String::from("6379"),
            db_text_input_state: text_input::State::default(),
            db_value: String::from("0"),
        };

        let keys_refresh_button_state = button::State::default();
        let tab_buttons = Vec::<(String, Index, button::State)>::new();
        let new_tab_button = button::State::default();
        let create_key_button = button::State::default();

        RedisViewer {
            server_tabs,
            current_server_tab_index,
            conn_form_state,
            keys_refresh_button_state,
            tab_buttons,
            new_tab_button,
            create_key_button,
        }
    }

    fn title(&self) -> String {
        String::from("Icy Redis Viewer")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::KeySelected(key) => {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");
                let value = get_redis_value(&mut current_server_tab.redis, &key)
                    .expect("failed to get value for selected redis key");
                match value {
                    RedisValue::String(s) => {
                        current_server_tab.editor_state = EditorState::Edit(ValueEditState {
                            key: key.clone(),
                            value: s,
                            ..default()
                        });
                    }
                    _ => {
                        current_server_tab.editor_state = EditorState::Empty;
                    }
                }
            }
            Message::SelectedValueChanged(s) => {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");
                match &mut current_server_tab.editor_state {
                    EditorState::Empty => {}
                    EditorState::Edit(edit_state) => {
                        edit_state.value = s;
                    }
                    EditorState::Create(_) => {}
                }
            }
            Message::SelectedValueSaved => {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");
                match &mut current_server_tab.editor_state {
                    EditorState::Empty => {}
                    EditorState::Edit(edit_state) => {
                        set_redis_value(
                            &mut current_server_tab.redis,
                            edit_state.key.clone(),
                            edit_state.value.clone(),
                        )
                        .expect("failed to set redis value");
                    }
                    EditorState::Create(_) => {}
                }
            }
            Message::SelectedValueDeleted => {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");

                match &mut current_server_tab.editor_state {
                    EditorState::Empty => {}
                    EditorState::Edit(edit_state) => {
                        delete_redis_key(&mut current_server_tab.redis, edit_state.key.clone())
                            .expect("failed to delete key");
                        current_server_tab.editor_state = EditorState::Empty;
                    }
                    EditorState::Create(_) => {}
                }
                self.refresh_keys();
            }
            Message::ConnNameChanged(s) => {
                self.conn_form_state.conn_name_value = s;
            }
            Message::ConnValueChanged(s) => {
                self.conn_form_state.conn_value = s;
            }
            Message::PortValueChanged(s) => {
                self.conn_form_state.port_value = s;
            }
            Message::DbValueChanged(s) => {
                self.conn_form_state.db_value = s;
            }
            Message::ConnectRedis => {
                let conn = self.conn_form_state.conn_value.clone();
                let port: u16 = self
                    .conn_form_state
                    .port_value
                    .parse()
                    .expect("failed to parse port");
                let db: i64 = self
                    .conn_form_state
                    .db_value
                    .parse()
                    .expect("failed to parse db");
                let mut redis =
                    connect_redis(&conn, port, db).expect("failed to get redis connection");
                let keys = get_all_keys(&mut redis).expect("failed to get keys");
                let namespaces = convert_keys_to_namespaces(&keys);
                let namespaces_view = create_namespace_views(&namespaces);
                let keys_scrollbar_state = KeysScrollbarState::default();
                let mut key_buttons = Vec::<(String, button::State)>::new();
                for key in keys.iter() {
                    key_buttons.push((key.clone(), button::State::default()));
                }

                let name = self.conn_form_state.conn_name_value.clone();

                let server_tab = ServerTab {
                    name,
                    redis,
                    keys,
                    keys_scrollbar_state,
                    namespaces,
                    key_buttons,
                    editor_state: EditorState::Empty,
                    namespaces_view,
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
                self.refresh_keys();
            }
            Message::ChangeTab(i) => {
                self.current_server_tab_index = Some(i);
            }
            Message::NewTab => {
                self.conn_form_state.show_connection_form = true;
                self.current_server_tab_index = None;
            }
            Message::OpenCreateKeyForm => {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");

                current_server_tab.editor_state = EditorState::Create(KeyCreateState::default());
            }
            Message::CreateKey => {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");

                match &mut current_server_tab.editor_state {
                    EditorState::Empty => {}
                    EditorState::Edit(_) => {}
                    EditorState::Create(state) => {
                        set_redis_value(
                            &mut current_server_tab.redis,
                            state.key.clone(),
                            state.value.clone(),
                        )
                        .expect("failed to set redis value");
                        current_server_tab.editor_state = EditorState::Empty;
                    }
                }
                self.refresh_keys();
            }
            Message::CreateKeyChanged(s) => {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");
                match &mut current_server_tab.editor_state {
                    EditorState::Empty => {}
                    EditorState::Edit(_) => {}
                    EditorState::Create(state) => {
                        state.key = s;
                    }
                }
            }
            Message::CreateValueChanged(s) => {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");
                match &mut current_server_tab.editor_state {
                    EditorState::Empty => {}
                    EditorState::Edit(_) => {}
                    EditorState::Create(state) => {
                        state.value = s;
                    }
                }
            }
            Message::NamespaceExpandToggle(indices) => {
                let current_server_tab = self
                    .server_tabs
                    .get_mut(
                        self.current_server_tab_index
                            .expect("failed to find current server tab index"),
                    )
                    .expect("failed to find current server tab in arena");
                let mut indices_iter = indices.iter();
                let indices_first = indices_iter.next();
                match indices_first {
                    Some(first_index) => {
                        let mut namespace_view =
                            current_server_tab.namespaces_view.get_mut(*first_index);
                        for i in indices_iter {
                            namespace_view = match namespace_view {
                                std::option::Option::Some(ns) => ns.sub_namespaces.get_mut(*i),
                                std::option::Option::None => None,
                            };
                        }
                        match namespace_view {
                            Some(ns) => {
                                ns.is_expanded = !ns.is_expanded;
                            }
                            None => {
                                println!("failed to find namespace view for index");
                            }
                        }
                    }
                    None => {
                        println!("no first index found in indices");
                    }
                }
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        let content = Column::new().align_items(Align::Center).spacing(20);

        let content = if self.conn_form_state.show_connection_form
            || self.current_server_tab_index == None
        {
            let connection_form = Column::new().push(
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
                    )
                    .push(
                        TextInput::new(
                            &mut self.conn_form_state.port_text_input_state,
                            "Enter the port for your redis server here.",
                            &self.conn_form_state.port_value,
                            Message::PortValueChanged,
                        )
                        .padding(5),
                    )
                    .push(
                        TextInput::new(
                            &mut self.conn_form_state.db_text_input_state,
                            "Enter the db for your redis server here.",
                            &self.conn_form_state.db_value,
                            Message::DbValueChanged,
                        )
                        .padding(5),
                    )
                    .push(
                        Row::new().padding(10).push(
                            Button::new(
                                &mut self.conn_form_state.connect_button,
                                Text::new("Connect"),
                            )
                            .on_press(Message::ConnectRedis),
                        ),
                    ),
            );

            content.push(connection_form)
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

            let keys = current_server_tab
                .namespaces_view
                .iter_mut()
                .enumerate()
                .fold(
                    Scrollable::new(&mut current_server_tab.keys_scrollbar_state.state)
                        .padding(0)
                        .align_items(Align::Start)
                        .width(Length::Fill)
                        .height(Length::Fill),
                    |scrollable, (i, ns)| {
                        let row = create_namespace_rows(ns, [i].to_vec());
                        scrollable.push(row)
                    },
                );

            let editor_column = Column::new()
                .align_items(Align::Start)
                .width(Length::FillPortion(3))
                .height(Length::Fill)
                .padding(20);

            let editor_column = match &mut current_server_tab.editor_state {
                EditorState::Empty => editor_column,
                EditorState::Edit(state) => editor_column
                    .push(Row::new().padding(20).push(Text::new(&state.key)))
                    .push(
                        Row::new().padding(20).push(
                            TextInput::new(
                                &mut state.value_input_state,
                                "Enter your redis value here.",
                                &state.value,
                                Message::SelectedValueChanged,
                            )
                            .width(Length::Fill)
                            .padding(10),
                        ),
                    )
                    .push(
                        Row::new()
                            .padding(20)
                            .push(
                                Button::new(&mut state.save_button_state, Text::new("Save"))
                                    .on_press(Message::SelectedValueSaved),
                            )
                            .push(
                                Button::new(&mut state.delete_button_state, Text::new("Delete"))
                                    .on_press(Message::SelectedValueDeleted),
                            ),
                    ),
                EditorState::Create(state) => editor_column
                    .push(
                        Row::new().padding(20).push(
                            TextInput::new(
                                &mut state.key_input_state,
                                "Enter your redis key here.",
                                &state.key,
                                Message::CreateKeyChanged,
                            )
                            .width(Length::Fill)
                            .padding(10),
                        ),
                    )
                    .push(
                        Row::new().padding(20).push(
                            TextInput::new(
                                &mut state.value_input_state,
                                "Enter your redis value here.",
                                &state.value,
                                Message::CreateValueChanged,
                            )
                            .width(Length::Fill)
                            .padding(10),
                        ),
                    )
                    .push(
                        Row::new().padding(20).push(
                            Button::new(&mut state.create_button_state, Text::new("Create"))
                                .on_press(Message::CreateKey),
                        ),
                    ),
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
                )
                .push(
                    Column::new().padding(5).push(
                        Button::new(&mut self.create_key_button, Text::new("New Key"))
                            .on_press(Message::OpenCreateKeyForm),
                    ),
                );

            let viewer_row = Row::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(10)
                .push(keys)
                .push(editor_column);

            content.push(tabs).push(tab_controls).push(viewer_row)
        };

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(style::Theme::Dark)
            .into()
    }

    fn background_color(&self) -> iced::Color {
        iced::Color::WHITE
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }
}
