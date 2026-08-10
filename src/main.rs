fn main() -> gtk::glib::ExitCode {
    data_inspector::i18n::init();
    data_inspector::application::run()
}
