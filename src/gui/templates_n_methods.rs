use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::borrow::Borrow;

use iced::alignment::{Horizontal, Vertical};
use iced::border::Radius;
use iced::widget::text_editor::{Action, Edit};
use iced::widget::{checkbox, text_editor, Column, Row, TextInput};
use iced::{Background, Pixels, Size, Task};
use iced::{widget::{button, container, pick_list, slider, text::{IntoFragment, Wrapping}, text_input, tooltip, Button, Container, Tooltip, column, row}, Border, Color, Length, Renderer, Shadow, Theme};
use num_traits::FromPrimitive;

use crate::gui::ice::{Exec};

const BG_MAIN: Color = Color::from_rgba8(181, 100, 255, 1.0);
const BG_SEC: Color = Color::from_rgba8(161, 105, 222, 1.0);
const BG_TRI: Color = Color::from_rgba8(141, 110, 190, 1.0);
const BG_QUAD: Color = Color::from_rgba8(122, 100, 160, 1.0);

const TEXT_WHITE: Color = Color::from_rgba8(222, 222, 222, 1.0);
const TEXT_GRAY: Color = Color::from_rgba8(100, 100, 100, 0.7);

const BORDER_ACTIVE: Color = Color::from_rgba8(92, 92, 92, 0.6);
const BORDER_HOVERED: Color = Color::from_rgba8(192, 192, 192, 0.8);

const RAD_MAIN: Radius = Radius { top_left: 2.0, top_right: 2.0, bottom_right: 2.0, bottom_left: 2.0 };
const RAD_SEC: Radius = Radius { top_left: 1.0, top_right: 1.0, bottom_right: 1.0, bottom_left: 1.0 };

const ENGLISH_CHARS: [char; 26] = ['A','B','C','D','E','F','G','H','I','J','K','L','M','N','O','P','Q','R','S','T','U','V','W','X','Y','Z'];

pub fn path_to_file_name(path: &Path) -> Option<String> {
    path.to_str()
        .map(|s| s.split("\\").collect::<Vec<&str>>() )
        .map(|p| p[p.len() - 1].to_string())
}

pub fn editor_paste(msg: &str) -> Action {
    text_editor::Action::Edit( Edit::Paste( Arc::new(msg.to_string()) ) )
}

pub fn padding_inner<'a, Exec>(width: impl Into<Length>) -> Container<'a, Exec, Theme, Renderer> {
    container("").width(width)
}

pub fn checkbox_default<'a, Exec, F> (
    checkbox_content: impl Into<String>,
    tooltip_content: impl Into<String>,
    checkbox_state: bool,
    tooltip_size: impl Into<Size>,
    exec: F,
) -> Row<'a, Exec>
where
    F: Fn(bool) -> Exec + 'a,
    Exec: 'a
{
    use iced::widget::checkbox::Status;
    use iced::widget::checkbox::Style;

    let tt_size: Size = tooltip_size.into();

    row![
        checkbox(checkbox_state)
            .label(checkbox_content.into())
            .on_toggle(exec)
            .size(17)
            .style(|_, status|
                Style {
                    background: Background::Color(match status {
                        Status::Active {is_checked} => { if is_checked { BG_SEC } else { Color::TRANSPARENT } },
                        Status::Hovered {is_checked} => { if is_checked { BG_MAIN } else { TEXT_GRAY }},
                        Status::Disabled {is_checked} => { if is_checked { TEXT_GRAY } else { Color::BLACK } },
                    }),
                    icon_color: Color::WHITE,
                    border: Border {
                        color: match status {
                            Status::Active {is_checked} => { if is_checked { BG_SEC } else { TEXT_GRAY } },
                            Status::Hovered {is_checked} => { if is_checked { BG_MAIN } else { TEXT_GRAY } },
                            Status::Disabled {is_checked} => { if is_checked { TEXT_GRAY } else { Color::TRANSPARENT } },
                        },
                        width: 1.0,
                        radius: RAD_MAIN
                    },
                    text_color: Some( TEXT_WHITE )
                }
            ),
        tooltip_default(tooltip_content, tt_size.width , tt_size.height),
    ].spacing(5).align_y( Vertical::Center ) //.width( Length::Fill ).spacing(5)
}

pub fn tooltip_default<'a, Exec> (
    content: impl Into<String>,
    x: impl Into<Length>,
    y: impl Into<Length>
) -> Tooltip<'a, Exec, Theme, Renderer> where Exec: 'a {
    tooltip(
        container( iced::widget::text("?").size(14.0))
            .center(17)
            .style( |_|
                container::Style {
                    text_color: Some( Color::WHITE ),
                    background: Some( Background::Color( Color::BLACK ) ),
                    border: Border { color: BG_SEC, width: 1.0, radius: RAD_SEC },
                    shadow: Shadow::default(),
                    snap: true,
                }
            ),
        container( iced::widget::text( content.into() ).size(15).center() )
            .padding(10)
            .center_x(x)
            .center_y(y)
            .style( |_|
                container::Style {
                    text_color: Some( TEXT_WHITE ),
                    background: Some( Background::Color( Color::BLACK ) ),
                    border: Border { color: BG_MAIN, width: 1.0, radius: RAD_MAIN },
                    shadow: Shadow::default(),
                    snap: true,
                }
            ),
        tooltip::Position::FollowCursor
    )
}

pub fn text_input_style() -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    use iced::widget::text_input::{Status, Style};

    |_, status| Style {
        background: Background::Color( Color::TRANSPARENT ),
        border: Border {
            color: match status {
                Status::Active => { BORDER_ACTIVE },
                Status::Hovered => { BORDER_HOVERED },
                Status::Focused{is_hovered: false} => { BG_MAIN },
                Status::Focused{is_hovered: true} => { BG_MAIN },
                Status::Disabled => { Color::TRANSPARENT }
            } ,
            width: 1.0,
            radius: RAD_MAIN
        },
        icon: Color::TRANSPARENT,
        placeholder: TEXT_GRAY,
        selection: BG_SEC,
        value: TEXT_WHITE
    }
}

pub fn text_input_default<'a, Exec, F>(
    text_input_content: impl Into<String>,
    text_input_value: Option<&String>,
    exec: F
) -> TextInput<'a, Exec>
where
    F: Fn(String) -> Exec + 'a,
    Exec: Clone + 'a
{
    text_input(
        &text_input_content.into(),
        text_input_value.unwrap_or( &String::from("") )
    ).on_input(exec)
    .style( text_input_style() )
    .width( Length::Fill)
}

pub fn btn_style_base() -> impl Fn(&Theme, button::Status) -> button::Style {
    |_, status: button::Status| button::Style {
        background: match status {
            button::Status::Active => Some( Background::Color(BG_MAIN) ),
            button::Status::Hovered => Some( Background::Color(BG_SEC) ),
            button::Status::Pressed => Some( Background::Color(BG_TRI) ),
            button::Status::Disabled => Some( Background::Color(BG_MAIN) ),
        },
        text_color: match status {
            button::Status::Active => Color::BLACK,
            button::Status::Hovered => Color::WHITE,
            button::Status::Pressed => Color::WHITE,
            button::Status::Disabled => Color::BLACK,
        },
        border: match status {
            button::Status::Active => Border { color: BG_SEC, width: 1.0, radius: RAD_MAIN },
            button::Status::Hovered => Border { color: BG_TRI, width: 1.0, radius: RAD_SEC },
            button::Status::Pressed => Border { color: BG_QUAD, width: 1.0, radius: RAD_SEC },
            button::Status::Disabled => Border { color: Color::BLACK, width: 0.0, radius: RAD_SEC },
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn btn_base<'a, Exec, D> (
    txt: impl Into<String>,
    font_size: Option<impl Into<Pixels>>,
    width: Option<D>,
    height: Option<D>,
    exec: Exec
) -> Button<'a, Exec>
where
    D: Into<Length>,
{
    let mut btn = button(
        iced::widget::text(txt.into() )
            .align_x( Horizontal::Center )
            .align_y( Vertical::Center )
            .size(if let Some(size) = font_size { size.into() } else { Pixels::from(16) })
        ).on_press(exec);

    btn = if let Some(w) = width { btn.width( w ) } else { btn };
    btn = if let Some(h) = height { btn.height( h ) } else { btn };
    btn.style( btn_style_base() )
}

pub fn text_editor_template<'a, Exec, F>(
    tooltip_content: impl Into<String>,
    field_name: impl Into<String>,
    field_placeholder: impl Into<String>,
    editor_state: &'a text_editor::Content,
    editor_height: impl Into<Length>,
    width: impl Into<Length>,
    tooltip_size: impl Into<Size>,
    exec: F,
) -> Column<'a, Exec>
where
    Exec: 'a + Clone,
    F: 'a + Fn(text_editor::Action) -> Exec + Copy,
{
    use iced::widget::text_editor::{Status, Style};

    let tt_size: Size = tooltip_size.into();

    column![
        row![
            tooltip_default(tooltip_content, tt_size.width, tt_size.height),
            iced::widget::text( field_name.into() ).width(Length::Fill).center(),
            padding_inner(30)
        ]
        .padding(2)
        .width(Length::Fill),

        text_editor(editor_state)
            .on_action(exec)
            .height(editor_height)
            .placeholder( field_placeholder.into() )
            .wrapping(Wrapping::WordOrGlyph)
            .style(|_, status|
                Style {
                    background: Background::Color( Color::TRANSPARENT ),
                    border: Border {
                        color: match status {
                            Status::Active => { BORDER_ACTIVE },
                            Status::Hovered => { BORDER_HOVERED },
                            Status::Focused{is_hovered: false} => { BG_MAIN },
                            Status::Focused{is_hovered: true} => { BG_MAIN },
                            Status::Disabled => { Color::TRANSPARENT }
                        } ,
                        width: 1.0,
                        radius: RAD_MAIN
                    },
                    placeholder: TEXT_GRAY,
                    value: TEXT_WHITE,
                    selection: BG_SEC
                }
            )
    ]
    .width(width)
}

pub fn pick_list_template<'a, T, L, V, F, Exec>(
    tooltip_content: impl Into<String>,
    field_name: impl Into<String>,
    options: &L,
    selected: Option<V>,
    on_selected: F,
    tooltip_size: impl Into<Size>,
    width: impl Into<Length>,
) -> Column<'a, Exec>
where
    Exec: Clone + 'a,
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + Clone + 'a,
    V: Borrow<T> + 'a,
    F: Fn(T) -> Exec + 'a,
{
    use pick_list::{Style, Status};
    use iced::overlay::menu;
    let size: Size = tooltip_size.into();

    column![
        row![
            tooltip_default(tooltip_content, size.width, size.height).padding(3),
            iced::widget::text(field_name.into()).width(Length::Fill).center(),
            padding_inner(30)
        ].spacing(5),
        pick_list(
            options.clone(),
            selected,
            on_selected
        ).width( Length::Fill )
        .style( |_, status| Style {
            text_color: match status {
                Status::Active => { TEXT_WHITE },
                Status::Hovered => { TEXT_WHITE },
                Status::Opened{is_hovered} => { if is_hovered { TEXT_WHITE } else { TEXT_GRAY } },
            },
            placeholder_color: TEXT_GRAY,
            handle_color: BG_TRI,
            background: Background::Color( Color::BLACK ),
            border: Border {
                color: match status {
                    Status::Active => { BORDER_ACTIVE },
                    Status::Hovered => { BORDER_HOVERED },
                    Status::Opened { is_hovered } => { if is_hovered { BG_MAIN } else { BORDER_ACTIVE } }
                },
                width: 1.0,
                radius: RAD_MAIN
            }
        })
        .menu_style(|_| menu::Style {
            background: Background::Color(Color::BLACK),
            border: Border {
                color: BG_MAIN,
                width: 1.0,
                radius: RAD_MAIN
            },
            text_color: TEXT_WHITE,
            selected_text_color: TEXT_WHITE,
            selected_background: Background::Color(BG_MAIN),
            shadow: Shadow::default()
        } )
        ,
    ].width( width )
    .padding(5)
    .spacing(5)
}

pub fn text_input_template<'a, F, Exec>(
    tooltip_content: impl Into<String>,
    tooltip_size: impl Into<Size>,
    field_name: impl Into<String>,
    field_placeholder: impl Into<String>,
    field_value: Option<&String>,
    on_input: F,
    width: impl Into<Length>
) -> Column<'a, Exec>
where
    Exec: Clone + 'a,
    F: Fn(String) -> Exec + 'a
{

    let tt_size: Size = tooltip_size.into();

    column![
        row![
            tooltip_default(tooltip_content, tt_size.width, tt_size.height),
            iced::widget::text( field_name.into() ).width(Length::Fill).center(),
            padding_inner(30),
        ].spacing(5),
        text_input_default(&field_placeholder.into(), field_value, on_input)
    ].width( width )
    .padding(5)
    .spacing(5)
}

pub fn slider_template<'a, T, F, G, Exec>(
    tooltip_content: impl Into<String>,
    field_name: impl Into<String>,
    tooltip_size: impl Into<Size>,
    range: std::ops::RangeInclusive<T>,
    value: T,
    text_value: &str,
    on_submit: F,
    on_input: G,
    width: impl Into<Length>
) -> Column<'a, Exec>
where
    T: From<u8> + Into<f64> + FromPrimitive + ToString + Copy + PartialOrd + IntoFragment<'a> + 'a,
    Exec: Clone + 'a,
    F: Fn(T) -> Exec + Clone + 'a,
    G: Fn((String, T, T)) -> Exec + 'a
{
    use iced::widget::slider::{Style, Status, Rail, Handle, HandleShape};

    let tt_size: Size = tooltip_size.into();
    let on_input_range = range.clone();

    column![
        row![
            tooltip_default(tooltip_content, tt_size.width, tt_size.height),
            iced::widget::text( field_name.into() ).width(Length::Fill).center(),
            padding_inner(20),
        ].spacing(5),
        slider(range.clone(), value, on_submit.clone())
            .style(|_, status| Style {
                rail: Rail {
                    backgrounds: (Background::Color( Color::TRANSPARENT ), Background::Color( Color::TRANSPARENT) ),
                    width: 5.0,
                    border: Border {
                        color: match status {
                            Status::Active => { BORDER_ACTIVE },
                            Status::Hovered => { BORDER_HOVERED },
                            Status::Dragged => { BG_MAIN }
                        },
                        width: 1.0,
                        radius: RAD_MAIN
                    }
                },
                handle: Handle {
                    shape: HandleShape::Circle { radius: 10.0 },
                    background: Background::Color( BG_MAIN ),
                    border_width: 2.5,
                    border_color: match status {
                        Status::Active => { BORDER_ACTIVE },
                        Status::Hovered => { BG_SEC },
                        Status::Dragged => { BG_MAIN }
                    }
                }
            }),
        text_input( &value.to_string(), text_value ).style( text_input_style() )
        .on_input(move |e| on_input((e, *on_input_range.start(), *on_input_range.end())))
        .on_submit_maybe(
            Some(
                on_submit( {
                    if value < *range.start() { *range.start() }
                    else if value > *range.end() { *range.end() }
                    else { value }
                } )
            )
        )
    ].width(width)
    .padding(5)
    .spacing(5)
}

pub trait ToNumeric {
    fn to_numeric<T>(&self) -> Result<T, T::Err>
    where
        T: FromStr;
}
pub trait ToOption {
    fn to_option<T>(&self) -> Option<T>
    where
        T: FromStr;
}

pub trait IsEnglishAlphabetic {
    fn is_english_alphabetic(&self) -> bool;
}

impl IsEnglishAlphabetic for char {
    fn is_english_alphabetic(&self) -> bool {
        ENGLISH_CHARS.contains(&self.to_ascii_uppercase())
    }
}

impl ToNumeric for String {
    fn to_numeric<T>(&self) -> Result<T, T::Err>
    where
        T: FromStr,
    {
        let mut res = String::with_capacity( self.len() );

        for char in self.chars() {
            if char.is_numeric() { res.push(char) }
        }
        res.parse::<T>()
    }
}
impl ToOption for str {
    fn to_option<T>(&self) -> Option<T>
    where
        T: FromStr
    {
        if self.trim().is_empty() { None } else { T::from_str(self).ok() }
    }
}

pub trait AssignFromStr {
    fn assign_from(&mut self, s: &str);
}

impl AssignFromStr for String {
    fn assign_from(&mut self, s: &str) {
        *self = s.to_string();
    }
}

impl AssignFromStr for Option<String> {
    fn assign_from(&mut self, s: &str) {
        *self = s.to_option()
    }
}

pub fn task_col_if_english_alphabetic<S: AssignFromStr>(state_val: &mut S, s: &str) -> Task<Exec> {
    if s.chars().any(|c| !c.is_english_alphabetic() ) { return Task::none() }
    state_val.assign_from(s);
    Task::none()
}

pub fn task_cell_if_english_alphabetic(state_val: &mut Option<String>, s: &str) -> Task<Exec> {
    if s.chars().any(|c| !c.is_english_alphabetic() && !c.is_ascii_digit() && c != '$' ) { return Task::none() }
    *state_val = s.to_option();
    Task::none()
}
