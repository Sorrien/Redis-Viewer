extern crate redis;
mod style;

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
    redis: redis::Connection,
    keys: Vec<String>,
    namespaces: HashMap<String, RedisNamespace>,
    keys_scrollbar_state: KeysScrollbarState,
    key_buttons: Vec<(String, button::State)>,
    selected_key: String,
    show_value_view: bool,
    selected_value: String,
    selected_value_text_input_state: text_input::State,
    save_button_state: button::State,
}

struct KeysScrollbarState {
    state: scrollable::State,
    //scrollbar_width: Option<u16>,
    //scrollbar_margin: Option<u16>,
    //scroller_width: Option<u16>,
}

#[derive(Debug, Clone)]
pub enum Message {
    KeySelected(usize),
    SelectedValueChanged(String),
    SelectedValueSaved,
}

impl Sandbox for RedisViewer {
    type Message = Message;

    fn new() -> Self {
        let mut redis =
            connect_redis("redis://127.0.0.1/").expect("failed to get redis connection");
        let keys = get_all_keys(&mut redis).expect("failed to get keys");
        let namespaces = convert_keys_to_namespaces(&keys);
        let keys_scrollbar_state = KeysScrollbarState {
            state: scrollable::State::new(),
            //scrollbar_width: Some(4),
            //scrollbar_margin: None,
            //scroller_width: Some(10),
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

        RedisViewer {
            redis,
            keys,
            namespaces,
            keys_scrollbar_state,
            key_buttons,
            selected_key,
            show_value_view,
            selected_value,
            selected_value_text_input_state,
            save_button_state,
        }
    }

    fn title(&self) -> String {
        String::from("Icy Redis Viewer")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::KeySelected(i) => {
                let key = self.keys[i].clone();
                self.selected_key = key.clone();
                self.selected_value = get_redis_value(&mut self.redis, key)
                    .expect("failed to get value for selected redis key");
                self.show_value_view = true;
            }
            Message::SelectedValueChanged(s) => {
                self.selected_value = s;
            }
            Message::SelectedValueSaved => {
                set_redis_value(
                    &mut self.redis,
                    self.selected_key.clone(),
                    self.selected_value.clone(),
                )
                .expect("failed to set redis value");
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        let keys = self.key_buttons.iter_mut().enumerate().fold(
            Scrollable::new(&mut self.keys_scrollbar_state.state)
                .padding(0)
                .align_items(Align::Start)
                .width(Length::FillPortion(2))
                .height(Length::Fill),
            |scrollable, (i, (key, state))| {
                scrollable.push(
                    Button::new(state, Text::new(key.clone())).on_press(Message::KeySelected(i)),
                )
            },
        );

        let value_column = if self.show_value_view {
            Column::new()
                .align_items(Align::Start)
                .width(Length::FillPortion(3))
                .height(Length::Fill)
                //.padding(20)
                .push(Row::new().padding(20).push(Text::new(&self.selected_key)))
                .push(
                    Row::new().padding(20).push(
                        TextInput::new(
                            &mut self.selected_value_text_input_state,
                            "Enter your redis value here.",
                            &self.selected_value,
                            Message::SelectedValueChanged,
                        )
                        .width(Length::Fill)
                        .padding(10),
                    ),
                )
                .push(
                    Button::new(&mut self.save_button_state, Text::new("Save"))
                        .on_press(Message::SelectedValueSaved),
                )
        } else {
            Column::new()
                .align_items(Align::Start)
                .width(Length::FillPortion(3))
                .height(Length::Fill)
                .padding(20)
        };

        let content = Row::new()
            .align_items(Align::Center)
            .spacing(20)
            .push(keys)
            .push(value_column);

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
