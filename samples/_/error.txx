
import std;

struct Hello {
    name: std::string,
}

fn Hello::hello() {
    // correct
    // std::cout << "Hello " << self.name << std::endl;

    // error: should use `self.name`
    std::cout << "Hello " << name << std::endl;
}

fn main() {
    let a: Hello;

    // correct
    // a.name = "TXX"

    // error: use of uninitialized value
    a.hello();
}
