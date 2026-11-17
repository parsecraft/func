//! FizzBuzz with tests. Try: func run examples/fizzbuzz.rs 15  and  func test examples/fizzbuzz.rs

fn fizzbuzz(n: u32) -> String {
    match (n % 3, n % 5) {
        (0, 0) => "FizzBuzz".to_string(),
        (0, _) => "Fizz".to_string(),
        (_, 0) => "Buzz".to_string(),
        _ => n.to_string(),
    }
}

fn main() {
    let limit: u32 = std::env::args().nth(1).and_then(|arg| arg.parse().ok()).unwrap_or(100);
    for n in 1..=limit {
        println!("{}", fizzbuzz(n));
    }
}

#[test]
fn multiples_of_three_fizz() {
    assert_eq!(fizzbuzz(9), "Fizz");
}

#[test]
fn multiples_of_five_buzz() {
    assert_eq!(fizzbuzz(10), "Buzz");
}

#[test]
fn multiples_of_both_fizzbuzz() {
    assert_eq!(fizzbuzz(15), "FizzBuzz");
}

#[test]
fn other_numbers_print_themselves() {
    assert_eq!(fizzbuzz(7), "7");
}
