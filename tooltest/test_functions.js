function searchTargetFunction() {
    // This is a target function for semantic search testing
}

function anotherFunction() {
    // Another function that should not be the target
    searchTargetFunction(); // Calling the target function here
}

class SampleClass {
    methodOne() {
        // A sample method
    }
    
    methodTwo() {
        this.methodOne(); // Calling another method in the same class
    }
}