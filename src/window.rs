use std::{cell::RefCell, collections::HashSet, rc::Rc};

use adw::prelude::*;
use gtk::glib::types::StaticType;
use gtk::{Align, Orientation, gio, glib};
use serde_json::Value;

use crate::{
    document::{DataDocument, load_document},
    i18n::{gettext, ngettext, replace_named},
    icons,
};

#[derive(Debug, Clone)]
struct VisibleNode {
    label: String,
    display_path: String,
    pointer: String,
    node_type: String,
    summary: String,
    child_count: usize,
    depth: usize,
    is_container: bool,
}

#[derive(Debug, Clone)]
struct CsvSelection {
    row_index: usize,
    column_index: usize,
    column_name: String,
    value: String,
}

#[derive(Debug, Clone)]
enum DataSelection {
    Json(VisibleNode),
    Csv(CsvSelection),
}

#[derive(Default)]
struct AppState {
    document: Option<DataDocument>,
    expanded: HashSet<String>,
    visible_nodes: Vec<VisibleNode>,
    selected: Option<DataSelection>,
    query: String,
}

type SharedState = Rc<RefCell<AppState>>;

#[derive(Clone)]
struct SidebarUi {
    toolbar: adw::ToolbarView,
    open_button: gtk::Button,
    close_button: gtk::Button,
    file_row: adw::ActionRow,
    format_row: adw::ActionRow,
    size_row: adw::ActionRow,
    nodes_row: adw::ActionRow,
    depth_row: adw::ActionRow,
    extra_row: adw::ActionRow,
    header_row: adw::ActionRow,
    header_switch: gtk::Switch,
}

#[derive(Clone)]
struct InspectorUi {
    toolbar: adw::ToolbarView,
    type_row: adw::ActionRow,
    path_row: adw::ActionRow,
    key_row: adw::ActionRow,
    children_row: adw::ActionRow,
    value_row: adw::ActionRow,
    copy_path_button: gtk::Button,
    copy_value_button: gtk::Button,
}

#[derive(Clone)]
struct ContentUi {
    split: adw::OverlaySplitView,
    stack: gtk::Stack,
    structured_stack: gtk::Stack,
    structured_list: gtk::ListBox,
    csv_model: gtk::StringList,
    csv_view: gtk::ColumnView,
    raw_buffer: gtk::TextBuffer,
    header_title: adw::WindowTitle,
    search_bar: gtk::Box,
    search_entry: gtk::SearchEntry,
    left_sidebar_button: gtk::ToggleButton,
    inspector_button: gtk::ToggleButton,
    structured_toggle: gtk::ToggleButton,
    raw_toggle: gtk::ToggleButton,
    inspector: InspectorUi,
    toast_overlay: adw::ToastOverlay,
}

pub fn build_window(app: &adw::Application) -> adw::ApplicationWindow {
    install_css();

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Data Inspector")
        .default_width(1180)
        .default_height(780)
        .build();
    window.set_size_request(560, 520);

    let toast_overlay = adw::ToastOverlay::new();
    window.set_content(Some(&toast_overlay));

    let outer_split = adw::OverlaySplitView::new();
    outer_split.set_sidebar_width_fraction(0.32);
    outer_split.set_min_sidebar_width(290.0);
    outer_split.set_max_sidebar_width(370.0);
    toast_overlay.set_child(Some(&outer_split));

    let sidebar = build_sidebar();
    outer_split.set_sidebar(Some(&sidebar.toolbar));

    let content = build_content(&toast_overlay);
    outer_split.set_content(Some(&content.split));

    outer_split
        .bind_property("show-sidebar", &content.left_sidebar_button, "active")
        .bidirectional()
        .sync_create()
        .build();

    content
        .split
        .bind_property("show-sidebar", &content.inspector_button, "active")
        .bidirectional()
        .sync_create()
        .build();

    let narrow = adw::Breakpoint::new(
        adw::BreakpointCondition::parse("max-width: 760sp")
            .expect("valid Data Inspector sidebar breakpoint"),
    );
    let outer_apply = outer_split.clone();
    narrow.connect_apply(move |_| {
        outer_apply.set_collapsed(true);
    });
    let outer_unapply = outer_split.clone();
    narrow.connect_unapply(move |_| {
        outer_unapply.set_collapsed(false);
    });
    window.add_breakpoint(narrow);

    let inspector_breakpoint = adw::Breakpoint::new(
        adw::BreakpointCondition::parse("max-width: 1040sp")
            .expect("valid Data Inspector inspector breakpoint"),
    );
    let inspector_split_apply = content.split.clone();
    inspector_breakpoint.connect_apply(move |_| {
        inspector_split_apply.set_collapsed(true);
        inspector_split_apply.set_show_sidebar(false);
    });
    let inspector_split_unapply = content.split.clone();
    let inspector_button_unapply = content.inspector_button.clone();
    inspector_breakpoint.connect_unapply(move |_| {
        inspector_split_unapply.set_collapsed(false);
        if inspector_button_unapply.is_sensitive() {
            inspector_split_unapply.set_show_sidebar(true);
        }
    });
    window.add_breakpoint(inspector_breakpoint);

    let state = Rc::new(RefCell::new(AppState::default()));
    connect_file_actions(&window, &toast_overlay, &sidebar, &content, &state);
    install_shortcuts(app, &window, &sidebar, &content);
    connect_search(&content, &state);
    connect_csv_header_override(&sidebar, &content, &state);
    connect_view_modes(&content);
    connect_tree_selection(&content, &state);
    connect_copy_actions(&toast_overlay, &content, &state);
    refresh_all(&sidebar, &content, &state);

    window
}

fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(
        r#"
.data-inspector-tree row {
    border-bottom: 1px solid alpha(@window_fg_color, 0.06);
}

.json-key {
    font-weight: 600;
}

.json-summary,
.json-type {
    color: alpha(@window_fg_color, 0.62);
}

.json-type {
    font-size: 0.88em;
}

.raw-data {
    font-family: monospace;
}
"#,
    );

    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_sidebar() -> SidebarUi {
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&adw::WindowTitle::new(
        "Data Inspector",
        &gettext("Structured data inspector"),
    )));

    let app_menu = gio::Menu::new();
    app_menu.append(Some(&gettext("About Data Inspector")), Some("app.about"));
    app_menu.append(Some(&gettext("Quit")), Some("app.quit"));
    let menu_button = gtk::MenuButton::builder()
        .menu_model(&app_menu)
        .icon_name(icons::MAIN_MENU)
        .tooltip_text(gettext("Main menu"))
        .build();
    header.pack_end(&menu_button);
    toolbar.add_top_bar(&header);

    let sidebar_box = gtk::Box::new(Orientation::Vertical, 0);
    toolbar.set_content(Some(&sidebar_box));

    let page = adw::PreferencesPage::new();
    page.set_vexpand(true);
    sidebar_box.append(&page);

    let file_group = adw::PreferencesGroup::builder()
        .title(gettext("Data file"))
        .build();
    page.add(&file_group);

    let file_row = adw::ActionRow::builder()
        .title(gettext("Current file"))
        .subtitle(gettext("No data file loaded"))
        .build();
    let close_button = gtk::Button::builder()
        .icon_name(icons::CURRENT_FILE_CLOSE)
        .tooltip_text(gettext("Close file"))
        .valign(Align::Center)
        .sensitive(false)
        .build();
    close_button.add_css_class("flat");

    let close_hover = gtk::EventControllerMotion::new();
    let close_button_enter = close_button.clone();
    close_hover.connect_enter(move |_, _, _| {
        close_button_enter.remove_css_class("flat");
        close_button_enter.add_css_class("destructive-action");
    });
    let close_button_leave = close_button.clone();
    close_hover.connect_leave(move |_| {
        close_button_leave.remove_css_class("destructive-action");
        close_button_leave.add_css_class("flat");
    });
    close_button.add_controller(close_hover);

    file_row.add_suffix(&close_button);
    file_group.add(&file_row);

    let open_button = gtk::Button::with_label(&gettext("Open Data"));
    open_button.add_css_class("suggested-action");
    open_button.set_margin_top(8);
    file_group.add(&open_button);

    let overview_group = adw::PreferencesGroup::builder()
        .title(gettext("Overview"))
        .build();
    page.add(&overview_group);

    let format_row = adw::ActionRow::builder()
        .title(gettext("Format"))
        .subtitle("—")
        .build();
    overview_group.add(&format_row);
    let size_row = adw::ActionRow::builder()
        .title(gettext("File size"))
        .subtitle("—")
        .build();
    overview_group.add(&size_row);
    let nodes_row = adw::ActionRow::builder()
        .title(gettext("JSON nodes"))
        .subtitle("—")
        .build();
    overview_group.add(&nodes_row);
    let depth_row = adw::ActionRow::builder()
        .title(gettext("Maximum depth"))
        .subtitle("—")
        .build();
    overview_group.add(&depth_row);
    let extra_row = adw::ActionRow::builder()
        .title(gettext("Delimiter"))
        .subtitle("—")
        .build();
    extra_row.set_visible(false);
    overview_group.add(&extra_row);

    let header_row = adw::ActionRow::builder()
        .title(gettext("First row is header"))
        .subtitle(gettext("Override automatic header detection"))
        .build();
    let header_switch = gtk::Switch::builder().valign(Align::Center).build();
    header_row.add_suffix(&header_switch);
    header_row.set_activatable_widget(Some(&header_switch));
    header_row.set_visible(false);
    overview_group.add(&header_row);

    SidebarUi {
        toolbar,
        open_button,
        close_button,
        file_row,
        format_row,
        size_row,
        nodes_row,
        depth_row,
        extra_row,
        header_row,
        header_switch,
    }
}

fn build_content(toast_overlay: &adw::ToastOverlay) -> ContentUi {
    let split = adw::OverlaySplitView::new();
    split.set_sidebar_position(gtk::PackType::End);
    split.set_sidebar_width_fraction(0.31);
    split.set_min_sidebar_width(290.0);
    split.set_max_sidebar_width(380.0);
    split.set_show_sidebar(false);

    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let header_title = adw::WindowTitle::new(
        "Data Inspector",
        &gettext("Open a file to inspect its data"),
    );
    header.set_title_widget(Some(&header_title));

    let left_sidebar_button = gtk::ToggleButton::builder()
        .icon_name(icons::LEFT_SIDEBAR)
        .tooltip_text(gettext("Show or hide left sidebar"))
        .build();
    header.pack_start(&left_sidebar_button);

    let view_switcher = gtk::Box::new(Orientation::Horizontal, 0);
    view_switcher.add_css_class("linked");
    let structured_toggle = gtk::ToggleButton::builder()
        .icon_name(icons::STRUCTURED_VIEW)
        .tooltip_text(gettext("Structured view"))
        .build();
    structured_toggle.set_active(true);
    let raw_toggle = gtk::ToggleButton::builder()
        .icon_name(icons::RAW_VIEW)
        .tooltip_text(gettext("Raw view"))
        .build();
    raw_toggle.set_group(Some(&structured_toggle));
    structured_toggle.set_sensitive(false);
    raw_toggle.set_sensitive(false);
    view_switcher.append(&structured_toggle);
    view_switcher.append(&raw_toggle);

    let inspector_button = gtk::ToggleButton::builder()
        .icon_name(icons::RIGHT_SIDEBAR)
        .build();
    inspector_button.set_tooltip_text(Some(&gettext("Show or hide Inspector")));
    inspector_button.set_sensitive(false);
    header.pack_end(&inspector_button);
    header.pack_end(&view_switcher);
    toolbar.add_top_bar(&header);

    let search_bar = gtk::Box::new(Orientation::Horizontal, 0);
    search_bar.set_margin_top(8);
    search_bar.set_margin_bottom(8);
    search_bar.set_margin_start(12);
    search_bar.set_margin_end(12);
    search_bar.set_visible(false);
    let search_entry = gtk::SearchEntry::new();
    search_entry.set_placeholder_text(Some(&gettext("Search data…")));
    search_entry.set_hexpand(true);
    search_entry.set_sensitive(false);
    search_bar.append(&search_entry);
    toolbar.add_top_bar(&search_bar);

    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    toolbar.set_content(Some(&stack));

    let empty_page = adw::StatusPage::builder()
        .icon_name(icons::OPEN_DOCUMENT)
        .title(gettext("Open a data file"))
        .description(gettext(
            "Drop a JSON, CSV, or TSV file here, or use Open Data in the sidebar.",
        ))
        .vexpand(true)
        .build();
    stack.add_named(&empty_page, Some("empty"));

    let structured_stack = gtk::Stack::new();

    let structured_scroll = gtk::ScrolledWindow::new();
    structured_scroll.set_hexpand(true);
    structured_scroll.set_vexpand(true);
    let structured_list = gtk::ListBox::new();
    structured_list.add_css_class("data-inspector-tree");
    structured_list.set_selection_mode(gtk::SelectionMode::Single);
    structured_list.set_margin_top(12);
    structured_list.set_margin_bottom(12);
    structured_list.set_margin_start(12);
    structured_list.set_margin_end(12);
    structured_scroll.set_child(Some(&structured_list));
    structured_stack.add_named(&structured_scroll, Some("json"));

    let csv_model = gtk::StringList::new(&[]);
    let csv_sort_model = gtk::SortListModel::new(Some(csv_model.clone()), None::<gtk::Sorter>);
    csv_sort_model.set_incremental(true);
    let csv_selection = gtk::SingleSelection::new(Some(csv_sort_model.clone()));
    let csv_view = gtk::ColumnView::new(Some(csv_selection));
    csv_sort_model.set_sorter(csv_view.sorter().as_ref());
    csv_view.set_hexpand(true);
    csv_view.set_vexpand(true);
    csv_view.set_show_row_separators(true);
    csv_view.set_show_column_separators(true);
    csv_view.set_reorderable(false);
    csv_view.add_css_class("data-table");

    let csv_scroll = gtk::ScrolledWindow::new();
    csv_scroll.set_hexpand(true);
    csv_scroll.set_vexpand(true);
    csv_scroll.set_child(Some(&csv_view));
    install_middle_mouse_panning(&csv_view, &csv_scroll);
    structured_stack.add_named(&csv_scroll, Some("csv"));

    stack.add_named(&structured_stack, Some("structured"));

    let raw_scroll = gtk::ScrolledWindow::new();
    raw_scroll.set_hexpand(true);
    raw_scroll.set_vexpand(true);
    let raw_buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
    let raw_view = gtk::TextView::with_buffer(&raw_buffer);
    raw_view.add_css_class("raw-data");
    raw_view.set_editable(false);
    raw_view.set_cursor_visible(true);
    raw_view.set_wrap_mode(gtk::WrapMode::None);
    raw_view.set_top_margin(18);
    raw_view.set_bottom_margin(18);
    raw_view.set_left_margin(18);
    raw_view.set_right_margin(18);
    raw_scroll.set_child(Some(&raw_view));
    stack.add_named(&raw_scroll, Some("raw"));
    stack.set_visible_child_name("empty");

    split.set_content(Some(&toolbar));

    let inspector = build_inspector();
    split.set_sidebar(Some(&inspector.toolbar));

    ContentUi {
        split,
        stack,
        structured_stack,
        structured_list,
        csv_model,
        csv_view,
        raw_buffer,
        header_title,
        search_bar,
        search_entry,
        left_sidebar_button,
        inspector_button,
        structured_toggle,
        raw_toggle,
        inspector,
        toast_overlay: toast_overlay.clone(),
    }
}

fn install_middle_mouse_panning(view: &gtk::ColumnView, scroll: &gtk::ScrolledWindow) {
    let gesture = gtk::GestureDrag::new();
    gesture.set_button(gtk::gdk::BUTTON_MIDDLE);

    let origin = Rc::new(RefCell::new((0.0_f64, 0.0_f64)));
    let origin_for_begin = origin.clone();
    let scroll_for_begin = scroll.clone();
    gesture.connect_drag_begin(move |_, _, _| {
        *origin_for_begin.borrow_mut() = (
            scroll_for_begin.hadjustment().value(),
            scroll_for_begin.vadjustment().value(),
        );
    });

    let origin_for_update = origin;
    let scroll_for_update = scroll.clone();
    gesture.connect_drag_update(move |_, offset_x, offset_y| {
        let (start_x, start_y) = *origin_for_update.borrow();
        set_panned_adjustment(&scroll_for_update.hadjustment(), start_x, offset_x);
        set_panned_adjustment(&scroll_for_update.vadjustment(), start_y, offset_y);
    });

    view.add_controller(gesture);
}

fn set_panned_adjustment(adjustment: &gtk::Adjustment, start: f64, offset: f64) {
    let lower = adjustment.lower();
    let upper = (adjustment.upper() - adjustment.page_size()).max(lower);
    adjustment.set_value((start - offset).clamp(lower, upper));
}

fn build_inspector() -> InspectorUi {
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&adw::WindowTitle::new(
        &gettext("Inspector"),
        &gettext("Selected data item"),
    )));
    toolbar.add_top_bar(&header);

    let root = gtk::Box::new(Orientation::Vertical, 0);
    toolbar.set_content(Some(&root));
    let page = adw::PreferencesPage::new();
    page.set_vexpand(true);
    root.append(&page);

    let group = adw::PreferencesGroup::builder()
        .title(gettext("Selection"))
        .build();
    page.add(&group);

    let type_row = adw::ActionRow::builder()
        .title(gettext("Type"))
        .subtitle("—")
        .build();
    group.add(&type_row);
    let path_row = adw::ActionRow::builder()
        .title(gettext("Path"))
        .subtitle("—")
        .build();
    group.add(&path_row);
    let key_row = adw::ActionRow::builder()
        .title(gettext("Key / index"))
        .subtitle("—")
        .build();
    group.add(&key_row);
    let children_row = adw::ActionRow::builder()
        .title(gettext("Children"))
        .subtitle("—")
        .build();
    group.add(&children_row);
    let value_row = adw::ActionRow::builder()
        .title(gettext("Value"))
        .subtitle("—")
        .build();
    group.add(&value_row);

    let actions_group = adw::PreferencesGroup::new();
    page.add(&actions_group);
    let actions = gtk::Box::new(Orientation::Horizontal, 6);
    actions.set_homogeneous(true);
    actions.set_margin_top(8);
    let copy_path_button = gtk::Button::with_label(&gettext("Copy Path"));
    let copy_value_button = gtk::Button::with_label(&gettext("Copy Value"));
    copy_path_button.add_css_class("suggested-action");
    copy_value_button.add_css_class("suggested-action");
    copy_path_button.set_sensitive(false);
    copy_value_button.set_sensitive(false);
    actions.append(&copy_path_button);
    actions.append(&copy_value_button);
    actions_group.add(&actions);

    InspectorUi {
        toolbar,
        type_row,
        path_row,
        key_row,
        children_row,
        value_row,
        copy_path_button,
        copy_value_button,
    }
}

fn install_shortcuts(
    app: &adw::Application,
    window: &adw::ApplicationWindow,
    sidebar: &SidebarUi,
    content: &ContentUi,
) {
    let open_action = gio::SimpleAction::new("open", None);
    let open_button = sidebar.open_button.clone();
    open_action.connect_activate(move |_, _| {
        open_button.emit_clicked();
    });
    window.add_action(&open_action);
    app.set_accels_for_action("win.open", &["<primary>o"]);

    let search_action = gio::SimpleAction::new("search", None);
    let search_entry = content.search_entry.clone();
    search_action.connect_activate(move |_, _| {
        if search_entry.is_sensitive() {
            search_entry.grab_focus();
        }
    });
    window.add_action(&search_action);
    app.set_accels_for_action("win.search", &["<primary>f"]);

    let clear_action = gio::SimpleAction::new("clear", None);
    let close_button = sidebar.close_button.clone();
    clear_action.connect_activate(move |_, _| {
        if close_button.is_sensitive() {
            close_button.emit_clicked();
        }
    });
    window.add_action(&clear_action);
    app.set_accels_for_action("win.clear", &["<primary><shift>x"]);
}

fn connect_file_actions(
    window: &adw::ApplicationWindow,
    toast_overlay: &adw::ToastOverlay,
    sidebar: &SidebarUi,
    content: &ContentUi,
    state: &SharedState,
) {
    let open_button = sidebar.open_button.clone();
    let window_for_open = window.clone();
    let toast_for_open = toast_overlay.clone();
    let sidebar_for_open = sidebar.clone();
    let content_for_open = content.clone();
    let state_for_open = state.clone();
    open_button.connect_clicked(move |_| {
        let dialog = gtk::FileDialog::builder()
            .title(gettext("Open Data"))
            .modal(true)
            .build();
        let (filter, filters) = data_filter();
        dialog.set_filters(Some(&filters));
        dialog.set_default_filter(Some(&filter));

        let window = window_for_open.clone();
        let toast = toast_for_open.clone();
        let sidebar = sidebar_for_open.clone();
        let content = content_for_open.clone();
        let state = state_for_open.clone();
        glib::MainContext::default().spawn_local(async move {
            let Ok(file) = dialog.open_future(Some(&window)).await else {
                return;
            };
            let Some(path) = file.path() else {
                return;
            };
            load_data(&path, &toast, &sidebar, &content, &state);
        });
    });

    let close_button = sidebar.close_button.clone();
    let sidebar_for_close = sidebar.clone();
    let content_for_close = content.clone();
    let state_for_close = state.clone();
    close_button.connect_clicked(move |_| {
        {
            let mut app_state = state_for_close.borrow_mut();
            *app_state = AppState::default();
        }
        content_for_close.search_entry.set_text("");
        refresh_all(&sidebar_for_close, &content_for_close, &state_for_close);
    });

    let drop_target = gtk::DropTarget::new(
        gtk::gdk::FileList::static_type(),
        gtk::gdk::DragAction::COPY,
    );
    let toast_for_drop = toast_overlay.clone();
    let sidebar_for_drop = sidebar.clone();
    let content_for_drop = content.clone();
    let state_for_drop = state.clone();
    drop_target.connect_drop(move |_, value, _, _| {
        let Ok(file_list) = value.get::<gtk::gdk::FileList>() else {
            return false;
        };
        let Some(path) = file_list.files().into_iter().find_map(|file| file.path()) else {
            return false;
        };
        load_data(
            &path,
            &toast_for_drop,
            &sidebar_for_drop,
            &content_for_drop,
            &state_for_drop,
        );
        true
    });
    content.split.add_controller(drop_target);
}

fn connect_search(content: &ContentUi, state: &SharedState) {
    let entry = content.search_entry.clone();
    let content = content.clone();
    let state = state.clone();
    entry.connect_search_changed(move |entry| {
        state.borrow_mut().query = entry.text().trim().to_lowercase();
        refresh_structured(&content, &state);
    });
}

fn connect_csv_header_override(sidebar: &SidebarUi, content: &ContentUi, state: &SharedState) {
    let header_switch = sidebar.header_switch.clone();
    let sidebar = sidebar.clone();
    let content = content.clone();
    let state = state.clone();
    header_switch.connect_active_notify(move |header_switch| {
        let changed = {
            let mut app_state = state.borrow_mut();
            let Some(DataDocument::Csv(document)) = app_state.document.as_mut() else {
                return;
            };
            if document.has_headers == header_switch.is_active() {
                false
            } else {
                document.set_has_headers(header_switch.is_active());
                app_state.selected = None;
                true
            }
        };

        if changed {
            refresh_all(&sidebar, &content, &state);
        }
    });
}

fn connect_view_modes(content: &ContentUi) {
    let stack = content.stack.clone();
    content.structured_toggle.connect_toggled(move |button| {
        if button.is_active() {
            stack.set_visible_child_name("structured");
        }
    });

    let stack = content.stack.clone();
    content.raw_toggle.connect_toggled(move |button| {
        if button.is_active() {
            stack.set_visible_child_name("raw");
        }
    });
}

fn connect_tree_selection(content: &ContentUi, state: &SharedState) {
    let list = content.structured_list.clone();
    let inspector = content.inspector.clone();
    let state = state.clone();
    list.connect_row_activated(move |_, row| {
        let index = row.index();
        if index < 0 {
            return;
        }
        let selected = state.borrow().visible_nodes.get(index as usize).cloned();
        let Some(selected) = selected else {
            return;
        };
        state.borrow_mut().selected = Some(DataSelection::Json(selected));
        refresh_inspector(&inspector, &state);
    });
}

fn connect_copy_actions(
    toast_overlay: &adw::ToastOverlay,
    content: &ContentUi,
    state: &SharedState,
) {
    let state_for_path = state.clone();
    let toast_for_path = toast_overlay.clone();
    content
        .inspector
        .copy_path_button
        .connect_clicked(move |_| {
            let Some(selected) = state_for_path.borrow().selected.clone() else {
                return;
            };
            let copied = match selected {
                DataSelection::Json(selected) => copy_text(&selected.display_path),
                DataSelection::Csv(selected) => {
                    let location = replace_named(
                        gettext("Row {row} · {column}"),
                        &[
                            ("row", (selected.row_index + 1).to_string()),
                            ("column", selected.column_name),
                        ],
                    );
                    copy_text(&location)
                }
            };
            if copied {
                show_toast(&toast_for_path, gettext("Path copied"));
            }
        });

    let state_for_value = state.clone();
    let toast_for_value = toast_overlay.clone();
    content
        .inspector
        .copy_value_button
        .connect_clicked(move |_| {
            let app_state = state_for_value.borrow();
            let (Some(document), Some(selected)) = (&app_state.document, &app_state.selected)
            else {
                return;
            };

            let copied = match (document, selected) {
                (DataDocument::Json(document), DataSelection::Json(selected)) => {
                    let Some(value) = document.value.pointer(&selected.pointer) else {
                        return;
                    };
                    copy_text(&copyable_value(value))
                }
                (DataDocument::Csv(_), DataSelection::Csv(selected)) => copy_text(&selected.value),
                _ => false,
            };
            if copied {
                show_toast(&toast_for_value, gettext("Value copied"));
            }
        });
}

fn json_context_action_group(
    state: &SharedState,
    toast_overlay: &adw::ToastOverlay,
) -> gio::SimpleActionGroup {
    let actions = gio::SimpleActionGroup::new();

    let copy_item = gio::SimpleAction::new("copy-item", None);
    let state_for_item = state.clone();
    let toast_for_item = toast_overlay.clone();
    copy_item.connect_activate(move |_, _| {
        let app_state = state_for_item.borrow();
        let (Some(DataDocument::Json(document)), Some(DataSelection::Json(selected))) =
            (&app_state.document, &app_state.selected)
        else {
            return;
        };
        let Some(value) = document.value.pointer(&selected.pointer) else {
            return;
        };
        let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
        if copy_text(&text) {
            show_toast(&toast_for_item, gettext("Item copied"));
        }
    });
    actions.add_action(&copy_item);

    let copy_path = gio::SimpleAction::new("copy-path", None);
    let state_for_path = state.clone();
    let toast_for_path = toast_overlay.clone();
    copy_path.connect_activate(move |_, _| {
        let app_state = state_for_path.borrow();
        let Some(DataSelection::Json(selected)) = &app_state.selected else {
            return;
        };
        if copy_text(&selected.display_path) {
            show_toast(&toast_for_path, gettext("Path copied"));
        }
    });
    actions.add_action(&copy_path);

    actions
}

fn csv_context_action_group(
    state: &SharedState,
    toast_overlay: &adw::ToastOverlay,
) -> gio::SimpleActionGroup {
    let actions = gio::SimpleActionGroup::new();

    let copy_cell = gio::SimpleAction::new("copy-cell", None);
    let state_for_cell = state.clone();
    let toast_for_cell = toast_overlay.clone();
    copy_cell.connect_activate(move |_, _| {
        let app_state = state_for_cell.borrow();
        let Some(DataSelection::Csv(selected)) = &app_state.selected else {
            return;
        };
        if copy_text(&selected.value) {
            show_toast(&toast_for_cell, gettext("Cell copied"));
        }
    });
    actions.add_action(&copy_cell);

    let copy_row = gio::SimpleAction::new("copy-row", None);
    let state_for_row = state.clone();
    let toast_for_row = toast_overlay.clone();
    copy_row.connect_activate(move |_, _| {
        let app_state = state_for_row.borrow();
        let (Some(DataDocument::Csv(document)), Some(DataSelection::Csv(selected))) =
            (&app_state.document, &app_state.selected)
        else {
            return;
        };
        let Some(row) = document.rows.get(selected.row_index) else {
            return;
        };
        if copy_text(&row.join("\t")) {
            show_toast(&toast_for_row, gettext("Row copied"));
        }
    });
    actions.add_action(&copy_row);

    let copy_column = gio::SimpleAction::new("copy-column", None);
    let state_for_column = state.clone();
    let toast_for_column = toast_overlay.clone();
    copy_column.connect_activate(move |_, _| {
        let app_state = state_for_column.borrow();
        let (Some(DataDocument::Csv(document)), Some(DataSelection::Csv(selected))) =
            (&app_state.document, &app_state.selected)
        else {
            return;
        };

        let header_capacity = if document.has_headers { 1 } else { 0 };
        let mut values = Vec::with_capacity(document.rows.len() + header_capacity);
        if document.has_headers {
            values.push(
                document
                    .headers
                    .get(selected.column_index)
                    .cloned()
                    .unwrap_or_default(),
            );
        }
        values.extend(
            document
                .rows
                .iter()
                .map(|row| row.get(selected.column_index).cloned().unwrap_or_default()),
        );

        if copy_text(&values.join("\n")) {
            show_toast(&toast_for_column, gettext("Column copied"));
        }
    });
    actions.add_action(&copy_column);

    actions
}

fn point_context_popover(popover: &gtk::PopoverMenu, x: f64, y: f64) {
    let rect = gtk::gdk::Rectangle::new(x.round() as i32, y.round() as i32, 1, 1);
    popover.set_pointing_to(Some(&rect));
    popover.popup();
}

fn data_filter() -> (gtk::FileFilter, gio::ListStore) {
    let filter = gtk::FileFilter::new();
    filter.set_name(Some(&gettext("Structured data files")));
    for pattern in ["*.json", "*.JSON", "*.csv", "*.CSV", "*.tsv", "*.TSV"] {
        filter.add_pattern(pattern);
    }
    filter.add_mime_type("application/json");
    filter.add_mime_type("text/csv");
    filter.add_mime_type("text/tab-separated-values");

    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    (filter, filters)
}

fn load_data(
    path: &std::path::Path,
    toast_overlay: &adw::ToastOverlay,
    sidebar: &SidebarUi,
    content: &ContentUi,
    state: &SharedState,
) {
    let document = match load_document(path) {
        Ok(document) => document,
        Err(error) => {
            show_toast(toast_overlay, error);
            return;
        }
    };

    let format_name = document.format_name();
    let initial_selection = match &document {
        DataDocument::Json(document) => Some(DataSelection::Json(node_info(
            &gettext("Root"),
            "$",
            "",
            &document.value,
            0,
        ))),
        DataDocument::Csv(_) => None,
    };

    {
        let mut app_state = state.borrow_mut();
        app_state.document = Some(document);
        app_state.expanded.clear();
        app_state.expanded.insert(String::new());
        app_state.query.clear();
        app_state.selected = initial_selection;
    }

    content.search_entry.set_text("");
    refresh_all(sidebar, content, state);
    show_toast(
        toast_overlay,
        replace_named(
            gettext("{format} loaded locally."),
            &[("format", format_name.to_string())],
        ),
    );
}

fn refresh_all(sidebar: &SidebarUi, content: &ContentUi, state: &SharedState) {
    refresh_sidebar(sidebar, state);

    let csv_has_headers = {
        let app_state = state.borrow();
        match app_state.document.as_ref() {
            Some(DataDocument::Csv(document)) => Some(document.has_headers),
            _ => None,
        }
    };
    if let Some(has_headers) = csv_has_headers {
        sidebar.header_switch.set_active(has_headers);
    }

    let has_document = state.borrow().document.is_some();
    sidebar.close_button.set_sensitive(has_document);
    sidebar.close_button.remove_css_class("destructive-action");
    sidebar.close_button.add_css_class("flat");
    content.search_bar.set_visible(has_document);
    content.search_entry.set_sensitive(has_document);
    content.structured_toggle.set_sensitive(has_document);
    content.raw_toggle.set_sensitive(has_document);
    content.inspector_button.set_sensitive(has_document);

    let document_view = {
        let app_state = state.borrow();
        app_state.document.as_ref().map(|document| {
            (
                document
                    .path()
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| gettext("Data file")),
                document.raw_text().to_string(),
                document.format_name(),
            )
        })
    };

    if let Some((filename, raw_text, format_name)) = document_view {
        content.header_title.set_title(&filename);
        content.header_title.set_subtitle(&replace_named(
            gettext("Structured {format} view"),
            &[("format", format_name.to_string())],
        ));
        content.raw_buffer.set_text(&raw_text);
        content.structured_toggle.set_active(true);
        content.stack.set_visible_child_name("structured");
        content.split.set_show_sidebar(true);
        refresh_structured(content, state);
        refresh_inspector(&content.inspector, state);
    } else {
        content.header_title.set_title("Data Inspector");
        content
            .header_title
            .set_subtitle(&gettext("Open a file to inspect its data"));
        content.raw_buffer.set_text("");
        content.stack.set_visible_child_name("empty");
        content.split.set_show_sidebar(false);
        clear_listbox(&content.structured_list);
        clear_csv_view(content);
        refresh_inspector(&content.inspector, state);
    }
}

fn refresh_sidebar(sidebar: &SidebarUi, state: &SharedState) {
    let app_state = state.borrow();
    let Some(document) = app_state.document.as_ref() else {
        sidebar
            .file_row
            .set_subtitle(&gettext("No data file loaded"));
        sidebar.format_row.set_subtitle("—");
        sidebar.size_row.set_subtitle("—");
        sidebar.nodes_row.set_title(&gettext("Items"));
        sidebar.nodes_row.set_subtitle("—");
        sidebar.depth_row.set_title(&gettext("Details"));
        sidebar.depth_row.set_subtitle("—");
        sidebar.extra_row.set_visible(false);
        sidebar.header_row.set_visible(false);
        return;
    };

    let fallback_file_name = gettext("Data file");
    sidebar.file_row.set_subtitle(
        document
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&fallback_file_name),
    );
    sidebar
        .size_row
        .set_subtitle(&human_bytes(document.size_bytes()));

    match document {
        DataDocument::Json(document) => {
            sidebar.format_row.set_subtitle(&replace_named(
                gettext("JSON · {type}"),
                &[("type", value_type(&document.value))],
            ));
            sidebar.nodes_row.set_title(&gettext("JSON nodes"));
            sidebar
                .nodes_row
                .set_subtitle(&document.node_count.to_string());
            sidebar.depth_row.set_title(&gettext("Maximum depth"));
            sidebar
                .depth_row
                .set_subtitle(&document.max_depth.to_string());
            sidebar.extra_row.set_visible(false);
            sidebar.header_row.set_visible(false);
        }
        DataDocument::Csv(document) => {
            let csv_format = if document.has_headers {
                gettext("CSV · headers detected")
            } else {
                gettext("CSV · generated column names")
            };
            sidebar.format_row.set_subtitle(&csv_format);
            sidebar.nodes_row.set_title(&gettext("Rows"));
            sidebar
                .nodes_row
                .set_subtitle(&document.rows.len().to_string());
            sidebar.depth_row.set_title(&gettext("Columns"));
            sidebar
                .depth_row
                .set_subtitle(&document.headers.len().to_string());
            sidebar.extra_row.set_visible(true);
            sidebar.extra_row.set_title(&gettext("Delimiter"));
            sidebar
                .extra_row
                .set_subtitle(&csv_delimiter_label(document.delimiter));
            sidebar.header_row.set_visible(true);
        }
    }
}

fn refresh_structured(content: &ContentUi, state: &SharedState) {
    let format = {
        let app_state = state.borrow();
        match app_state.document.as_ref() {
            Some(DataDocument::Json(_)) => Some("json"),
            Some(DataDocument::Csv(_)) => Some("csv"),
            None => None,
        }
    };

    match format {
        Some("json") => {
            content.structured_stack.set_visible_child_name("json");
            refresh_tree_json(content, state);
        }
        Some("csv") => {
            content.structured_stack.set_visible_child_name("csv");
            configure_csv_columns(content, state);
            refresh_csv_model(content, state);
        }
        _ => {}
    }
}

fn refresh_tree_json(content: &ContentUi, state: &SharedState) {
    let nodes = {
        let app_state = state.borrow();
        match app_state.document.as_ref() {
            Some(DataDocument::Json(document)) => collect_visible_nodes(
                &document.value,
                &app_state.expanded,
                app_state.query.as_str(),
            ),
            _ => Vec::new(),
        }
    };

    state.borrow_mut().visible_nodes = nodes.clone();
    clear_listbox(&content.structured_list);

    let searching = !state.borrow().query.is_empty();
    for node in nodes {
        let row = gtk::ListBoxRow::new();
        row.set_activatable(true);
        row.set_selectable(true);

        let box_row = gtk::Box::new(Orientation::Horizontal, 8);
        box_row.set_margin_top(8);
        box_row.set_margin_bottom(8);
        box_row.set_margin_start(10 + (node.depth as i32 * 18));
        box_row.set_margin_end(12);

        if node.is_container && !searching {
            let expanded = state.borrow().expanded.contains(&node.pointer);
            let toggle_tooltip = if expanded {
                gettext("Collapse")
            } else {
                gettext("Expand")
            };
            let toggle = gtk::Button::builder()
                .icon_name(if expanded {
                    icons::DISCLOSURE_EXPANDED
                } else {
                    icons::DISCLOSURE_COLLAPSED
                })
                .tooltip_text(toggle_tooltip)
                .valign(Align::Center)
                .build();
            toggle.add_css_class("flat");
            toggle.set_focus_on_click(false);
            let pointer = node.pointer.clone();
            let content_for_toggle = content.clone();
            let state_for_toggle = state.clone();
            toggle.connect_clicked(move |_| {
                {
                    let mut app_state = state_for_toggle.borrow_mut();
                    if !app_state.expanded.insert(pointer.clone()) {
                        app_state.expanded.remove(&pointer);
                    }
                }
                refresh_tree_json(&content_for_toggle, &state_for_toggle);
            });
            box_row.append(&toggle);
        } else {
            let spacer = gtk::Box::new(Orientation::Horizontal, 0);
            spacer.set_size_request(34, 1);
            box_row.append(&spacer);
        }

        let text_box = gtk::Box::new(Orientation::Vertical, 2);
        text_box.set_hexpand(true);
        let key = gtk::Label::builder()
            .label(&node.label)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        key.add_css_class("json-key");
        text_box.append(&key);

        let summary = gtk::Label::builder()
            .label(&node.summary)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        summary.add_css_class("json-summary");
        text_box.append(&summary);
        box_row.append(&text_box);

        let type_label = gtk::Label::new(Some(&node.node_type));
        type_label.add_css_class("json-type");
        type_label.set_valign(Align::Center);
        box_row.append(&type_label);

        row.set_child(Some(&box_row));

        let menu = gio::Menu::new();
        menu.append(Some(&gettext("Copy Item")), Some("context.copy-item"));
        menu.append(Some(&gettext("Copy Path")), Some("context.copy-path"));
        let actions = json_context_action_group(state, &content.toast_overlay);
        row.insert_action_group("context", Some(&actions));
        let popover = gtk::PopoverMenu::from_model(Some(&menu));
        popover.set_has_arrow(false);
        popover.set_parent(&row);

        let secondary_click = gtk::GestureClick::new();
        secondary_click.set_button(gtk::gdk::BUTTON_SECONDARY);
        let list_for_context = content.structured_list.downgrade();
        let row_for_context = row.downgrade();
        let node_for_context = node.clone();
        let inspector_for_context = content.inspector.clone();
        let state_for_context = state.clone();
        let popover_for_context = popover.downgrade();
        secondary_click.connect_pressed(move |_, _, x, y| {
            let (Some(list), Some(row), Some(popover)) = (
                list_for_context.upgrade(),
                row_for_context.upgrade(),
                popover_for_context.upgrade(),
            ) else {
                return;
            };
            list.select_row(Some(&row));
            state_for_context.borrow_mut().selected =
                Some(DataSelection::Json(node_for_context.clone()));
            refresh_inspector(&inspector_for_context, &state_for_context);
            point_context_popover(&popover, x, y);
        });
        row.add_controller(secondary_click);

        content.structured_list.append(&row);
    }
}

fn configure_csv_columns(content: &ContentUi, state: &SharedState) {
    let (headers, preferred_widths) = {
        let app_state = state.borrow();
        let Some(DataDocument::Csv(document)) = app_state.document.as_ref() else {
            return;
        };
        let widths = (0..document.headers.len())
            .map(|column_index| csv_preferred_width_chars(document, column_index))
            .collect::<Vec<_>>();
        (document.headers.clone(), widths)
    };

    let columns = content.csv_view.columns();
    let current_count = columns.n_items() as usize;
    let headers_match = current_count == headers.len()
        && headers.iter().enumerate().all(|(index, header)| {
            columns
                .item(index as u32)
                .and_downcast::<gtk::ColumnViewColumn>()
                .and_then(|column| column.title())
                .is_some_and(|title| title.as_str() == header.as_str())
        });
    if headers_match && current_count > 0 {
        return;
    }

    while content.csv_view.columns().n_items() > 0 {
        let Some(column) = content
            .csv_view
            .columns()
            .item(0)
            .and_downcast::<gtk::ColumnViewColumn>()
        else {
            break;
        };
        content.csv_view.remove_column(&column);
    }

    for (column_index, header) in headers.into_iter().enumerate() {
        let preferred_width = preferred_widths.get(column_index).copied().unwrap_or(14);
        let factory = gtk::SignalListItemFactory::new();

        let state_for_setup = state.clone();
        let inspector_for_setup = content.inspector.clone();
        let toast_for_setup = content.toast_overlay.clone();
        let csv_view_for_setup = content.csv_view.clone();
        factory.connect_setup(move |_, object| {
            let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
                return;
            };

            let label = gtk::Label::builder()
                .xalign(0.0)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .width_chars(preferred_width)
                .max_width_chars(48)
                .margin_start(10)
                .margin_end(10)
                .margin_top(7)
                .margin_bottom(7)
                .build();
            label.set_selectable(false);

            let click = gtk::GestureClick::new();
            click.set_button(gtk::gdk::BUTTON_PRIMARY);
            let list_item_for_click = list_item.downgrade();
            let state_for_click = state_for_setup.clone();
            let inspector_for_click = inspector_for_setup.clone();
            click.connect_released(move |_, _, _, _| {
                let Some(list_item) = list_item_for_click.upgrade() else {
                    return;
                };
                let Some(item) = list_item.item().and_downcast::<gtk::StringObject>() else {
                    return;
                };
                let Ok(row_index) = item.string().parse::<usize>() else {
                    return;
                };

                let selection = {
                    let app_state = state_for_click.borrow();
                    let Some(DataDocument::Csv(document)) = app_state.document.as_ref() else {
                        return;
                    };
                    let Some(row) = document.rows.get(row_index) else {
                        return;
                    };
                    let Some(value) = row.get(column_index) else {
                        return;
                    };
                    let column_name =
                        document
                            .headers
                            .get(column_index)
                            .cloned()
                            .unwrap_or_else(|| {
                                replace_named(
                                    gettext("Column {index}"),
                                    &[("index", (column_index + 1).to_string())],
                                )
                            });
                    CsvSelection {
                        row_index,
                        column_index,
                        column_name,
                        value: value.clone(),
                    }
                };

                state_for_click.borrow_mut().selected = Some(DataSelection::Csv(selection));
                refresh_inspector(&inspector_for_click, &state_for_click);
            });
            label.add_controller(click);

            let menu = gio::Menu::new();
            menu.append(Some(&gettext("Copy Cell")), Some("context.copy-cell"));
            menu.append(Some(&gettext("Copy Row")), Some("context.copy-row"));
            menu.append(Some(&gettext("Copy Column")), Some("context.copy-column"));
            let actions = csv_context_action_group(&state_for_setup, &toast_for_setup);
            label.insert_action_group("context", Some(&actions));
            let popover = gtk::PopoverMenu::from_model(Some(&menu));
            popover.set_has_arrow(false);
            popover.set_parent(&label);

            let secondary_click = gtk::GestureClick::new();
            secondary_click.set_button(gtk::gdk::BUTTON_SECONDARY);
            let list_item_for_context = list_item.downgrade();
            let state_for_context = state_for_setup.clone();
            let inspector_for_context = inspector_for_setup.clone();
            let csv_view_for_context = csv_view_for_setup.downgrade();
            let popover_for_context = popover.downgrade();
            secondary_click.connect_pressed(move |_, _, x, y| {
                let Some(list_item) = list_item_for_context.upgrade() else {
                    return;
                };
                let Some(item) = list_item.item().and_downcast::<gtk::StringObject>() else {
                    return;
                };
                let Ok(row_index) = item.string().parse::<usize>() else {
                    return;
                };

                let selection = {
                    let app_state = state_for_context.borrow();
                    let Some(DataDocument::Csv(document)) = app_state.document.as_ref() else {
                        return;
                    };
                    let Some(row) = document.rows.get(row_index) else {
                        return;
                    };
                    let Some(value) = row.get(column_index) else {
                        return;
                    };
                    let column_name =
                        document
                            .headers
                            .get(column_index)
                            .cloned()
                            .unwrap_or_else(|| {
                                replace_named(
                                    gettext("Column {index}"),
                                    &[("index", (column_index + 1).to_string())],
                                )
                            });
                    CsvSelection {
                        row_index,
                        column_index,
                        column_name,
                        value: value.clone(),
                    }
                };

                let (Some(csv_view), Some(popover)) = (
                    csv_view_for_context.upgrade(),
                    popover_for_context.upgrade(),
                ) else {
                    return;
                };
                if let Some(model) = csv_view.model()
                    && let Ok(selection_model) = model.downcast::<gtk::SingleSelection>()
                {
                    selection_model.set_selected(list_item.position());
                }
                state_for_context.borrow_mut().selected = Some(DataSelection::Csv(selection));
                refresh_inspector(&inspector_for_context, &state_for_context);
                point_context_popover(&popover, x, y);
            });
            label.add_controller(secondary_click);

            list_item.set_child(Some(&label));
        });

        factory.connect_teardown(|_, object| {
            let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let Some(label) = list_item.child().and_downcast::<gtk::Label>() else {
                return;
            };

            // GtkPopoverMenu is manually attached to the cell label so it can
            // point at the right-click location. Undo that setup explicitly
            // before the recycled list item and label are destroyed.
            if let Some(child) = label.first_child()
                && let Ok(popover) = child.downcast::<gtk::PopoverMenu>()
            {
                popover.popdown();
                popover.unparent();
            }
        });

        let state_for_bind = state.clone();
        factory.connect_bind(move |_, object| {
            let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let Some(label) = list_item.child().and_downcast::<gtk::Label>() else {
                return;
            };
            let Some(item) = list_item.item().and_downcast::<gtk::StringObject>() else {
                label.set_label("");
                return;
            };
            let Ok(row_index) = item.string().parse::<usize>() else {
                label.set_label("");
                return;
            };

            let value = {
                let app_state = state_for_bind.borrow();
                let Some(DataDocument::Csv(document)) = app_state.document.as_ref() else {
                    return;
                };
                document
                    .rows
                    .get(row_index)
                    .and_then(|row| row.get(column_index))
                    .cloned()
                    .unwrap_or_default()
            };
            if value.is_empty() {
                label.set_label("—");
                label.add_css_class("dim-label");
                label.set_tooltip_text(Some(&gettext("Empty cell")));
            } else {
                label.set_label(&value);
                label.remove_css_class("dim-label");
                label.set_tooltip_text(Some(value.as_str()));
            }
        });

        let state_for_sort = state.clone();
        let sorter = gtk::CustomSorter::new(move |left, right| {
            let Some(left) = left.downcast_ref::<gtk::StringObject>() else {
                return std::cmp::Ordering::Equal.into();
            };
            let Some(right) = right.downcast_ref::<gtk::StringObject>() else {
                return std::cmp::Ordering::Equal.into();
            };
            let Ok(left_index) = left.string().parse::<usize>() else {
                return std::cmp::Ordering::Equal.into();
            };
            let Ok(right_index) = right.string().parse::<usize>() else {
                return std::cmp::Ordering::Equal.into();
            };

            let app_state = state_for_sort.borrow();
            let Some(DataDocument::Csv(document)) = app_state.document.as_ref() else {
                return std::cmp::Ordering::Equal.into();
            };
            let left_value = document
                .rows
                .get(left_index)
                .and_then(|row| row.get(column_index))
                .map(String::as_str)
                .unwrap_or("");
            let right_value = document
                .rows
                .get(right_index)
                .and_then(|row| row.get(column_index))
                .map(String::as_str)
                .unwrap_or("");
            compare_csv_values(left_value, right_value).into()
        });

        let column = gtk::ColumnViewColumn::builder()
            .title(header)
            .factory(&factory)
            .resizable(true)
            .build();
        column.set_sorter(Some(&sorter));
        content.csv_view.append_column(&column);
    }
}

fn csv_preferred_width_chars(document: &crate::document::CsvDocument, column_index: usize) -> i32 {
    let header_len = document
        .headers
        .get(column_index)
        .map(|header| header.chars().count())
        .unwrap_or(0);
    let sample_len = document
        .rows
        .iter()
        .take(120)
        .filter_map(|row| row.get(column_index))
        .map(|value| {
            value
                .lines()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0);

    header_len.max(sample_len).clamp(8, 30) as i32
}

fn compare_csv_values(left: &str, right: &str) -> std::cmp::Ordering {
    let left = left.trim();
    let right = right.trim();

    match (left.is_empty(), right.is_empty()) {
        (true, false) => return std::cmp::Ordering::Less,
        (false, true) => return std::cmp::Ordering::Greater,
        _ => {}
    }

    if let (Ok(left_number), Ok(right_number)) = (left.parse::<f64>(), right.parse::<f64>())
        && let Some(ordering) = left_number.partial_cmp(&right_number)
    {
        return ordering;
    }

    if let (Some(left_bool), Some(right_bool)) = (csv_bool(left), csv_bool(right)) {
        return left_bool.cmp(&right_bool);
    }

    left.to_lowercase().cmp(&right.to_lowercase())
}

fn csv_bool(value: &str) -> Option<bool> {
    if value.eq_ignore_ascii_case("true") {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

fn csv_value_type(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        gettext("Empty")
    } else if csv_bool(value).is_some() {
        gettext("Boolean")
    } else if value.parse::<f64>().is_ok() {
        gettext("Number")
    } else {
        gettext("Text")
    }
}

fn refresh_csv_model(content: &ContentUi, state: &SharedState) {
    let visible_rows = {
        let app_state = state.borrow();
        match app_state.document.as_ref() {
            Some(DataDocument::Csv(document)) if app_state.query.is_empty() => {
                (0..document.rows.len()).collect::<Vec<_>>()
            }
            Some(DataDocument::Csv(document)) => document
                .rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| {
                    row.iter()
                        .any(|value| value.to_lowercase().contains(&app_state.query))
                        .then_some(index)
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        }
    };

    let strings = visible_rows
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let refs = strings.iter().map(String::as_str).collect::<Vec<_>>();
    content
        .csv_model
        .splice(0, content.csv_model.n_items(), &refs);
}

fn clear_csv_view(content: &ContentUi) {
    content
        .csv_model
        .splice(0, content.csv_model.n_items(), &[]);
    while content.csv_view.columns().n_items() > 0 {
        let Some(column) = content
            .csv_view
            .columns()
            .item(0)
            .and_downcast::<gtk::ColumnViewColumn>()
        else {
            break;
        };
        content.csv_view.remove_column(&column);
    }
}

fn refresh_inspector(inspector: &InspectorUi, state: &SharedState) {
    let app_state = state.borrow();
    let (Some(document), Some(selected)) = (&app_state.document, &app_state.selected) else {
        inspector.type_row.set_subtitle("—");
        inspector.path_row.set_title(&gettext("Path"));
        inspector.path_row.set_subtitle("—");
        inspector.key_row.set_title(&gettext("Key / index"));
        inspector.key_row.set_subtitle("—");
        inspector.children_row.set_title(&gettext("Children"));
        inspector.children_row.set_subtitle("—");
        inspector.value_row.set_subtitle("—");
        inspector.copy_path_button.set_label(&gettext("Copy Path"));
        inspector.copy_path_button.set_sensitive(false);
        inspector.copy_value_button.set_sensitive(false);
        return;
    };

    match (document, selected) {
        (DataDocument::Json(document), DataSelection::Json(selected)) => {
            let Some(value) = document.value.pointer(&selected.pointer) else {
                return;
            };
            inspector.type_row.set_subtitle(&selected.node_type);
            inspector.path_row.set_title(&gettext("Path"));
            inspector.path_row.set_subtitle(&selected.display_path);
            inspector.key_row.set_title(&gettext("Key / index"));
            inspector.key_row.set_subtitle(&selected.label);
            inspector.children_row.set_title(&gettext("Children"));
            inspector
                .children_row
                .set_subtitle(&selected.child_count.to_string());
            inspector.value_row.set_subtitle(&inspector_value(value));
            inspector.copy_path_button.set_label(&gettext("Copy Path"));
            inspector.copy_path_button.set_sensitive(true);
            inspector.copy_value_button.set_sensitive(true);
        }
        (DataDocument::Csv(_), DataSelection::Csv(selected)) => {
            inspector
                .type_row
                .set_subtitle(&csv_value_type(&selected.value));
            inspector.path_row.set_title(&gettext("Row"));
            inspector
                .path_row
                .set_subtitle(&(selected.row_index + 1).to_string());
            inspector.key_row.set_title(&gettext("Column"));
            inspector.key_row.set_subtitle(&selected.column_name);
            inspector.children_row.set_title(&gettext("Column index"));
            inspector
                .children_row
                .set_subtitle(&(selected.column_index + 1).to_string());
            inspector
                .value_row
                .set_subtitle(&truncate(&selected.value, 180));
            inspector
                .copy_path_button
                .set_label(&gettext("Copy Location"));
            inspector.copy_path_button.set_sensitive(true);
            inspector.copy_value_button.set_sensitive(true);
        }
        _ => {}
    }
}

fn csv_delimiter_label(delimiter: u8) -> String {
    match delimiter {
        b';' => gettext("Semicolon (;)"),
        b'\t' => gettext("Tab"),
        _ => gettext("Comma (,)"),
    }
}

fn collect_visible_nodes(
    root: &Value,
    expanded: &HashSet<String>,
    query: &str,
) -> Vec<VisibleNode> {
    let mut nodes = Vec::new();
    collect_node(
        root,
        gettext("Root"),
        "$".to_string(),
        String::new(),
        0,
        expanded,
        query,
        &mut nodes,
    );
    nodes
}

#[allow(clippy::too_many_arguments)]
fn collect_node(
    value: &Value,
    label: String,
    display_path: String,
    pointer: String,
    depth: usize,
    expanded: &HashSet<String>,
    query: &str,
    out: &mut Vec<VisibleNode>,
) {
    let node = node_info(&label, &display_path, &pointer, value, depth);
    let searching = !query.is_empty();
    let matches = !searching
        || node.label.to_lowercase().contains(query)
        || node.display_path.to_lowercase().contains(query)
        || node.node_type.to_lowercase().contains(query)
        || node.summary.to_lowercase().contains(query);

    if matches {
        out.push(node);
    }

    if searching || expanded.contains(&pointer) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    let child_path = json_path_for_key(&display_path, key);
                    let child_pointer = format!("{}/{}", pointer, pointer_escape(key));
                    collect_node(
                        child,
                        key.clone(),
                        child_path,
                        child_pointer,
                        depth + 1,
                        expanded,
                        query,
                        out,
                    );
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    collect_node(
                        child,
                        format!("[{index}]"),
                        format!("{display_path}[{index}]"),
                        format!("{pointer}/{index}"),
                        depth + 1,
                        expanded,
                        query,
                        out,
                    );
                }
            }
            _ => {}
        }
    }
}

fn node_info(
    label: &str,
    display_path: &str,
    pointer: &str,
    value: &Value,
    depth: usize,
) -> VisibleNode {
    VisibleNode {
        label: label.to_string(),
        display_path: display_path.to_string(),
        pointer: pointer.to_string(),
        node_type: value_type(value),
        summary: value_summary(value),
        child_count: child_count(value),
        depth,
        is_container: matches!(value, Value::Object(_) | Value::Array(_)),
    }
}

fn value_type(value: &Value) -> String {
    match value {
        Value::Null => gettext("Null"),
        Value::Bool(_) => gettext("Boolean"),
        Value::Number(_) => gettext("Number"),
        Value::String(_) => gettext("String"),
        Value::Array(_) => gettext("Array"),
        Value::Object(_) => gettext("Object"),
    }
}

fn value_summary(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => truncate(value, 96),
        Value::Array(items) => replace_named(
            ngettext("{count} item", "{count} items", items.len()),
            &[("count", items.len().to_string())],
        ),
        Value::Object(map) => replace_named(
            ngettext("{count} key", "{count} keys", map.len()),
            &[("count", map.len().to_string())],
        ),
    }
}

fn inspector_value(value: &Value) -> String {
    match value {
        Value::Array(items) => replace_named(
            ngettext(
                "Array with {count} item",
                "Array with {count} items",
                items.len(),
            ),
            &[("count", items.len().to_string())],
        ),
        Value::Object(map) => replace_named(
            ngettext(
                "Object with {count} key",
                "Object with {count} keys",
                map.len(),
            ),
            &[("count", map.len().to_string())],
        ),
        Value::String(value) => truncate(value, 180),
        _ => value.to_string(),
    }
}

fn copyable_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
        }
        _ => value.to_string(),
    }
}

fn child_count(value: &Value) -> usize {
    match value {
        Value::Array(items) => items.len(),
        Value::Object(map) => map.len(),
        _ => 0,
    }
}

fn json_path_for_key(parent: &str, key: &str) -> String {
    if is_simple_identifier(key) {
        format!("{parent}.{key}")
    } else {
        let quoted = serde_json::to_string(key).unwrap_or_else(|_| format!("\"{key}\""));
        format!("{parent}[{quoted}]")
    }
}

fn is_simple_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn clear_listbox(list: &gtk::ListBox) {
    while let Some(child) = list.first_child() {
        let Ok(row) = child.downcast::<gtk::ListBoxRow>() else {
            break;
        };

        // GtkPopoverMenu is manually attached to the JSON row so it can
        // point at the right-click location. Detach it explicitly before
        // the row is removed and destroyed.
        let mut child = row.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();

            if let Ok(popover) = widget.downcast::<gtk::PopoverMenu>() {
                popover.popdown();
                popover.unparent();
            }
        }

        list.remove(&row);
    }
}

fn copy_text(text: &str) -> bool {
    let Some(display) = gtk::gdk::Display::default() else {
        return false;
    };
    display.clipboard().set_text(text);
    true
}

fn show_toast(overlay: &adw::ToastOverlay, message: impl Into<String>) {
    let message = message.into();
    overlay.add_toast(adw::Toast::new(&message));
}

fn human_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    let bytes_f = bytes as f64;
    if bytes_f >= GIB {
        format!("{:.2} GB", bytes_f / GIB)
    } else if bytes_f >= MIB {
        format!("{:.2} MB", bytes_f / MIB)
    } else if bytes_f >= KIB {
        format!("{:.1} KB", bytes_f / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}
