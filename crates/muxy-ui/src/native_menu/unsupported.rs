use super::Item;

pub fn show(
    _items: Vec<Item>,
    _anchor: gpui::Point<gpui::Pixels>,
    _window: &mut gpui::Window,
    _cx: &mut gpui::App,
) -> Option<async_channel::Receiver<Option<usize>>> {
    None
}
