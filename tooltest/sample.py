# 示例Python文件用于测试semantic_search

class DataProcessor:
    def __init__(self):
        self.data = []
    
    def process_data(self, input_data):
        processed = {
            'id': len(self.data),
            'content': input_data,
            'status': 'processed'
        }
        self.data.append(processed)
        return processed

def calculate_sum(numbers):
    total = 0
    for num in numbers:
        total += num
    return total

CONFIGURATION = {
    'database_url': 'postgresql://localhost:5432/test',
    'debug': True,
    'max_connections': 100
}

# 使用示例
processor = DataProcessor()
result = processor.process_data("test content")
total = calculate_sum([1, 2, 3, 4, 5])