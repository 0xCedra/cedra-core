script {
    // The main function where the program starts execution
    fun main() {
        // Declare a variable y of type u64 and initialize it with a value of 100
        let y: u64 = 100;
        // Check if the value of y is less than or equal to 10, if true assign the value of (y+1) to y, otherwise assign the value of 10 to y
        let z = if (y <= 10) y = y + 1 else y = 10;
    }
}