use iced::time::milliseconds;
use iced::widget::{button, center_y, column, container, image, row, text, text_input};
use iced::window;
use iced::window::screenshot::{self, Screenshot};
use iced::{Center, ContentFit, Element, Fill, FillPortion, Rectangle, Subscription, Task};
use iced::{keyboard, time};

use ::image as img;
use ::image::ColorType;

fn main() -> iced::Result {
    // tracing_subscriber::fmt::init();

    iced::application(Example::default, Example::update, Example::view)
        .subscription(Example::subscription)
        .run()
}

#[derive(Default)]
struct Example {
    screenshot: Option<(Screenshot, image::Handle)>,
    saved_png_path: Option<Result<String, PngError>>,
    png_saving: bool,
    x_input_value: Option<u32>,
    y_input_value: Option<u32>,
    width_input_value: Option<u32>,
    height_input_value: Option<u32>,
    state: State,
    duration: std::time::Duration,
    string: String,
}

#[derive(Default, Debug)]
enum State {
    #[default]
    Idle,
    Ticking {
        last_tick: std::time::Instant,
    },
}

#[derive(Clone, Debug)]
enum Message {
    Screenshot,
    Screenshotted(Screenshot),
    Png,
    PngSaved(Result<String, PngError>),
    Exit,
    Url,
    DataFetched(String),

    XInputChanged(Option<u32>),
    YInputChanged(Option<u32>),
    WidthInputChanged(Option<u32>),
    HeightInputChanged(Option<u32>),
    Tick(std::time::Instant),
}

impl Example {
    fn default() -> Self {
        Self {
            state: State::Ticking {
                last_tick: std::time::Instant::now(),
            },
            ..Default::default()
        }
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DataFetched(data) => {
                self.string = data;
                return Task::done(Message::Screenshot);

            }
            Message::Tick(now) => {
                if let State::Ticking { last_tick } = &mut self.state {
                    self.duration += now - *last_tick;
                    *last_tick = now;

                    println!("Tick: {:?}", self.duration);

                    if self.duration.as_secs_f32() >= 2.0 {
                        self.state = State::Idle;
                        self.duration = std::time::Duration::ZERO;
                        return Task::done(Message::Url);
                        // return window::latest()
                        //     .and_then(window::screenshot)
                        //     .map(Message::Screenshotted);
                    }
                }
            }
            Message::Screenshot => {
                return window::latest()
                    .and_then(window::screenshot)
                    .map(Message::Screenshotted);
            }
            Message::Screenshotted(screenshot) => {
                self.screenshot = Some((
                    screenshot.clone(),
                    image::Handle::from_rgba(
                        screenshot.size.width,
                        screenshot.size.height,
                        screenshot.rgba,
                    ),
                ));
                return Task::done(Message::Png);
            }
            Message::Png => {
                if let Some((screenshot, _handle)) = &self.screenshot {
                    self.png_saving = true;

                    return Task::perform(save_to_png(screenshot.clone()), Message::PngSaved);
                }
            }
            Message::PngSaved(res) => {
                self.png_saving = false;
                self.saved_png_path = Some(res);
            }
            Message::XInputChanged(new_value) => {
                self.x_input_value = new_value;
            }
            Message::YInputChanged(new_value) => {
                self.y_input_value = new_value;
            }
            Message::WidthInputChanged(new_value) => {
                self.width_input_value = new_value;
            }
            Message::HeightInputChanged(new_value) => {
                self.height_input_value = new_value;
            }
            Message::Exit => {
                std::process::exit(0);
            }
            Message::Url => {
                return Task::perform(fetch_data(), |result| {
                    match result {
                        Ok(data) => Message::DataFetched(data),
                        Err(_) => Message::DataFetched("Failed to fetch data".to_string()),
                    }
                });
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let side_content = column![text!("{}", self.string).size(20),];

        let content = row![side_content]
            .spacing(10)
            .width(Fill)
            .height(Fill)
            .align_y(Center);

        container(content).padding(10).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        use keyboard::key;
        let tick = match self.state {
            State::Idle => Subscription::none(),
            State::Ticking { .. } => time::every(milliseconds(100)).map(Message::Tick),
        };

        Subscription::batch(vec![
            tick,
            keyboard::listen().filter_map(|event| {
                if let keyboard::Event::KeyPressed {
                    modified_key: keyboard::Key::Named(key::Named::F5),
                    ..
                } = event
                {
                    Some(Message::Screenshot)
                } else {
                    None
                }
            }),
        ])
    }
}

async fn save_to_png(screenshot: Screenshot) -> Result<String, PngError> {
    let path = "screenshot.png".to_string();

    let _ = tokio::task::spawn_blocking(move || {
        img::save_buffer(
            &path,
            &screenshot.rgba,
            screenshot.size.width,
            screenshot.size.height,
            ColorType::Rgba8,
        )
        .map(|_| path)
        .map_err(|error| PngError(error.to_string()))
    })
    .await
    .expect("Blocking task to finish");

    pimoroni_notify().await.expect("?");
    Ok("screenshot.png".to_string())
}

async fn pimoroni_notify() -> tokio::io::Result<()> {
    sh_exec("/home/david/onthoudhet-kiosk.sh").await
}

async fn sh_exec(command: &str) -> tokio::io::Result<()> {
    use tokio::process::Command;

    Command::new("bash")
        .arg("-c")
        .arg(command)
        .spawn()?
        .wait()
        .await
        .map(|_| ())
}

#[derive(Clone, Debug)]
struct PngError(String);

fn numeric_input(placeholder: &str, value: Option<u32>) -> Element<'_, Option<u32>> {
    text_input(
        placeholder,
        &value.as_ref().map(ToString::to_string).unwrap_or_default(),
    )
    .on_input(move |text| {
        if text.is_empty() {
            None
        } else if let Ok(new_value) = text.parse() {
            Some(new_value)
        } else {
            value
        }
    })
    .width(40)
    .into()
}

fn centered_text(content: &str) -> Element<'_, Message> {
    text(content).width(Fill).align_x(Center).into()
}

async fn fetch_data() -> Result<String, reqwest::Error> {
    let response = reqwest::get("https://onthoudhetv2.weilers.nl/")
        .await?
        .text()
        .await?;
    Ok(response)
}