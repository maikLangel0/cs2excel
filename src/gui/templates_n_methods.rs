use std::path::PathBuf;
use std::str::FromStr;
use std::{borrow::Borrow, sync::LazyLock};

use iced::alignment::{Horizontal, Vertical};
use iced::border::Radius;
use iced::widget::{text_editor, Column};
use iced::{Background, Size};
use iced::{widget::{button, container, pick_list, slider, text::{IntoFragment, Wrapping}, text_input, tooltip, Button, Container, Tooltip, column, row}, Border, Color, Length, Renderer, Shadow, Theme};
use num_traits::FromPrimitive;

static BG_MAIN: LazyLock<Color> = LazyLock::new(|| Color::from_rgba8(83 ,203 ,227, 1.0));
static BG_SEC: LazyLock<Color> = LazyLock::new(|| Color::from_rgba8(44, 194, 218, 1.0));
static BG_TRI: LazyLock<Color> = LazyLock::new(|| Color::from_rgba8(38, 170, 191, 1.0));
static BG_QUAD: LazyLock<Color> = LazyLock::new(|| Color::from_rgba8(28, 118, 133, 1.0));

const RAD_MAIN: Radius = Radius { top_left: 2.0, top_right: 2.0, bottom_right: 2.0, bottom_left: 2.0 };
const RAD_SEC: Radius = Radius { top_left: 1.0, top_right: 1.0, bottom_right: 1.0, bottom_left: 1.0 };

const ENGLISH_CHARS: [char; 26] = ['A','B','C','D','E','F','G','H','I','J','K','L','M','N','O','P','Q','R','S','T','U','V','W','X','Y','Z'];

pub fn path_to_file_name(path: &PathBuf) -> Option<String> {
    let p = path.to_str()
        .and_then(|s| Some (s.split("\\")
        .collect::<Vec<&str>>() ));

    match p {
        Some(p) => { Some(p[p.len() - 1].to_string()) }
        None => None
    }
}

pub fn padding_inner<'a, Exec>(width: impl Into<Length>) -> Container<'a, Exec, Theme, Renderer> {
    container("").width(width)
}

pub fn tooltip_default<'a, Exec> (  
    content: impl Into<String>,
    x: impl Into<Length>,
    y: impl Into<Length>
) -> Tooltip<'a, Exec, Theme, Renderer> where Exec: 'a {
    tooltip( 
        container( iced::widget::text( "?" ) )
            .center(25)
            .style( |_| 
                container::Style { 
                    text_color: Some( Color::WHITE ), 
                    background: Some( Background::Color( Color::from_rgba8(72, 76, 92, 1.0) ) ), 
                    border: Border { color: Color::from_rgba8(88, 98, 99, 1.0), width: 1.0, radius: RAD_SEC },
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
                    text_color: Some( Color::BLACK ), 
                    background: Some( Background::Color(*BG_MAIN) ), 
                    border: Border { color: *BG_TRI, width: 1.0, radius: RAD_MAIN }, 
                    shadow: Shadow::default(),
                    snap: true, 
                } 
            ), 
        tooltip::Position::FollowCursor
    )  
}

pub fn btn_style_base() -> impl Fn(&Theme, button::Status) -> button::Style {
    |_, status: button::Status| button::Style {
        background: match status {
            button::Status::Active => Some( Background::Color(*BG_MAIN) ),
            button::Status::Hovered => Some( Background::Color(*BG_SEC) ),
            button::Status::Pressed => Some( Background::Color(*BG_TRI) ),
            button::Status::Disabled => Some( Background::Color(*BG_MAIN) ),
        },
        text_color: match status {
            button::Status::Active => Color::BLACK,
            button::Status::Hovered => Color::WHITE,
            button::Status::Pressed => Color::WHITE,
            button::Status::Disabled => Color::BLACK,
        },
        border: match status {
            button::Status::Active => Border { color: *BG_SEC, width: 1.0, radius: RAD_MAIN },
            button::Status::Hovered => Border { color: *BG_TRI, width: 1.0, radius: RAD_SEC },
            button::Status::Pressed => Border { color: *BG_QUAD, width: 1.0, radius: RAD_SEC },
            button::Status::Disabled => Border { color: Color::BLACK, width: 0.0, radius: RAD_SEC },
        }, 
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn btn_base<'a, Exec, D> (
    txt: impl Into<String>, 
    width: Option<D>, 
    height: Option<D>,
    exec: Exec
) -> Button<'a, Exec> 
where 
    D: Into<Length>,
{
    let mut btn = button( iced::widget::text(txt.into() ).align_x( Horizontal::Center ).align_y( Vertical::Center ) ).on_press(exec);

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
    exec: F,
) -> Column<'a, Exec>
where
    Exec: 'a + Clone,
    F: 'a + Fn(text_editor::Action) -> Exec + Copy, 
{
    column![
        row![
            tooltip_default(tooltip_content, 400, 125),
            iced::widget::text( field_name.into() ).width(Length::Fill).center(),
            padding_inner(30)
        ]
        .padding(2)
        .width(Length::Fill),

        text_editor(editor_state)
            .on_action(move |act| exec(act))
            .height(editor_height)
            .placeholder( field_placeholder.into() )
            .wrapping(Wrapping::WordOrGlyph),
    ]
    .width(width)
}

pub fn pick_list_template<'a, T, L, V, F, Exec>(
    tooltip_content: impl Into<String>,
    field_name: impl Into<String>,
    options: L,
    selected: Option<V>,
    on_selected: F,
    width: impl Into<Length>,
) -> Column<'a, Exec>
where
    Exec: Clone + 'a,
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
    F: Fn(T) -> Exec + 'a,
{
    column![
        row![
            tooltip_default(tooltip_content, 400, 100).padding(3),
            iced::widget::text(field_name.into()).width(Length::Fill).center(),
            padding_inner(30)
        ].spacing(5),
        pick_list( 
            options, 
            selected, 
            on_selected
        ).width( Length::Fill ),
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
    let empty = &String::new();

    column![
        row![
            tooltip_default(tooltip_content, tt_size.width, tt_size.height),
            iced::widget::text( field_name.into() ).width(Length::Fill).center(),
            padding_inner(30),
        ].spacing(5),
        text_input(
            &field_placeholder.into(), 
            field_value.as_ref().unwrap_or_else(|| &empty)
        ).on_input(move |id| on_input(id))
        .width( Length::Fill)
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
    let tt_size: Size = tooltip_size.into();
    let on_input_range = range.clone();

    column![
        row![
            tooltip_default(tooltip_content, tt_size.width, tt_size.height),
            iced::widget::text( field_name.into() ).width(Length::Fill).center(),
            padding_inner(20),
        ].spacing(5),
        slider(range.clone(), value, on_submit.clone()),
        //iced::widget::text(value).align_x( Horizontal::Center ).width( Length::Fill ),
        text_input( &value.to_string(), text_value )
        .on_input(move |e| on_input((e, *on_input_range.start(), *on_input_range.end())))
        .on_submit_maybe(
            Some(
                ( ||{ 
                    on_submit( {
                        if value < *range.start() { *range.start() } 
                        else if value > *range.end() { *range.end() } 
                        else { value }
                    } )
                })()
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