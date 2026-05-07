trait Printer<T> {
    fn print(&self, value: T);
}

struct StderrPrinter;

impl Printer<(&str, &str)> for StderrPrinter {
    fn print(&self, value: (&str, &str)) {
        eprintln!("{}{}", value.0, value.1);
    }
}

fn main() {
    let printer = StderrPrinter;
    printer.print(("Hello, ", "World!"));
}
