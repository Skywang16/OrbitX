class Calculator
  attr_accessor :memory

  def initialize
    @memory = 0.0
  end

  def add(a, b)
    result = a + b
    @memory = result
    result
  end

  def multiply(a, b)
    result = a * b
    @memory = result
    result
  end

  private

  def clear_memory
    @memory = 0.0
  end
end

module MathUtils
  def self.format_number(n)
    "%.2f" % n
  end

  def self.is_even?(num)
    num % 2 == 0
  end
end

def main
  calc = Calculator.new
  puts "Addition: #{calc.add(5.0, 3.0)}"
  puts "Multiplication: #{calc.multiply(2.0, 4.0)}"
  puts "Formatted: #{MathUtils.format_number(calc.memory)}"
end

main if __FILE__ == $0