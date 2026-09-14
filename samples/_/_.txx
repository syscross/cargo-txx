
import std;

struct Hello {
    name: std::string,
}

fn Hello::hello() {
    std::cout << "Hello " << self.name << std::endl;
}

fn main() {
    let a: Hello;

    a.name = "TXX";

    a.hello();
}
