fn hw() ->String{
    "Hello, world!".to_string()
}


fn main() {
    println!("{}",hw());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        assert_eq!("Hello, world!" , hw());
    }
    #[test]
    #[should_panic]
    fn ne_it_works() {
        assert_eq!("Hello, WORLD!" , hw());
    }
}
