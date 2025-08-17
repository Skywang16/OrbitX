def search_target_function():
    """This is a target function for semantic search testing."""
    pass

def another_function():
    """Another function that should not be the target."""
    search_target_function()  # Calling the target function here

class SampleClass:
    def method_one(self):
        pass
    
    def method_two(self):
        self.method_one()  # Calling another method in the same class