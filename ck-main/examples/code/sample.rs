use std::collections::HashMap;

pub struct Calculator {
    memory: f64,
}

impl Calculator {
    pub fn new() -> Self {
        Calculator { memory: 0.0 }
    }

    pub fn add(&mut self, a: f64, b: f64) -> f64 {
        let result = a + b;
        self.memory = result;
        result
    }

    pub fn multiply(&mut self, a: f64, b: f64) -> f64 {
        let result = a * b;
        self.memory = result;
        result
    }
}

fn main() {
    let mut calc = Calculator::new();
    println!("Addition: {}", calc.add(5.0, 3.0));
    println!("Multiplication: {}", calc.multiply(2.0, 4.0));
}

pub mod utils {
    pub fn format_number(n: f64) -> String {
        format!("{:.2}", n)
    }
}

