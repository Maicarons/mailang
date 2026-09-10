# 面向对象编程

## 概述

MaìLang 支持完整的面向对象编程范式，包括：
- 类与继承
- 接口（Trait）
- 封装
- 多态

## 类

### 基本类定义

```
class Animal {
    // 属性
    let name: str
    let age: int

    // 构造函数
    fn init(name: str, age: int) {
        this.name = name
        this.age = age
    }

    // 方法
    fn speak() -> str {
        return "{this.name} 发出声音"
    }
    
    fn info() -> str {
        return "{this.name}, {this.age}岁"
    }
}

// 创建实例
let cat = Animal("小花", 3)
println(cat.speak())  // 小花 发出声音
println(cat.info())   // 小花, 3岁
```

### 属性访问

```
let dog = Animal("小黑", 2)
println(dog.name)  // 小黑
println(dog.age)   // 2

// 修改属性（需要 var）
var animal = Animal("小白", 1)
animal.name = "大白"
```

## 继承

### 基本继承

```
class Dog extends Animal {
    let breed: str

    fn init(name: str, age: int, breed: str) {
        super(name, age)  // 调用父类构造函数
        this.breed = breed
    }

    // 方法重写
    override fn speak() -> str {
        return "{this.name} 汪汪叫！"
    }
    
    // 子类特有方法
    fn fetch() -> str {
        return "{this.name} 去捡球"
    }
}

let dog = Dog("小黑", 3, "拉布拉多")
println(dog.speak())   // 小黑 汪汪叫！
println(dog.fetch())   // 小黑 去捡球
println(dog.info())    // 继承自 Animal: 小黑, 3岁
```

### 多层继承

```
class Puppy extends Dog {
    fn init(name: str, breed: str) {
        super(name, 0, breed)  // 小狗年龄为0
    }

    override fn speak() -> str {
        return "{this.name} 嗷呜~"
    }
}

let puppy = Puppy("豆豆", "柴犬")
println(puppy.speak())  // 豆豆 嗷呜~
```

## Trait（接口）

### 定义 Trait

```
trait Printable {
    // 必须实现的方法
    fn to_string() -> str
    
    // 默认实现
    fn print() {
        println(this.to_string())
    }
    
    fn debug() -> str {
        return "Printable: {this.to_string()}"
    }
}
```

### 实现 Trait

```
class Point implements Printable {
    let x: float
    let y: float

    fn init(x: float, y: float) {
        this.x = x
        this.y = y
    }

    fn to_string() -> str {
        return "({this.x}, {this.y})"
    }
}

let p = Point(3.0, 4.0)
p.print()           // (3, 4)
println(p.debug())  // Printable: (3, 4)
```

### 多 Trait 实现

```
trait Serializable {
    fn serialize() -> str
}

trait Deserializable {
    fn deserialize(data: str) -> Self
}

class Config implements Printable, Serializable {
    let name: str
    let value: str

    fn init(name: str, value: str) {
        this.name = name
        this.value = value
    }

    fn to_string() -> str {
        return "{this.name}={this.value}"
    }
    
    fn serialize() -> str {
        return "{this.name}:{this.value}"
    }
}
```

## 封装

### 公有与私有

默认情况下，所有属性和方法都是公有的。使用 `priv` 关键字标记私有成员：

```
class BankAccount {
    let owner: str
    priv var balance: float

    fn init(owner: str, initial: float) {
        this.owner = owner
        this.balance = initial
    }

    pub fn deposit(amount: float) {
        if amount > 0 {
            this.balance += amount
        }
    }

    pub fn withdraw(amount: float) -> bool {
        if amount > 0 && amount <= this.balance {
            this.balance -= amount
            return true
        }
        return false
    }

    pub fn get_balance() -> float {
        return this.balance
    }
}

let account = BankAccount("张三", 1000.0)
account.deposit(500.0)
println(account.get_balance())  // 1500.0
// account.balance  // 错误！私有属性
```

## 静态成员

```
class MathUtils {
    static const PI = 3.14159
    
    static fn square(x: int) -> int {
        return x * x
    }
    
    static fn circle_area(radius: float) -> float {
        return MathUtils.PI * radius * radius
    }
}

let area = MathUtils.circle_area(5.0)
let sq = MathUtils.square(4)
```

## 高级特性

### 抽象类

```
abstract class Shape {
    abstract fn area() -> float
    abstract fn perimeter() -> float
    
    fn describe() -> str {
        return "面积: {this.area()}, 周长: {this.perimeter()}"
    }
}

class Circle extends Shape {
    let radius: float
    
    fn init(radius: float) {
        this.radius = radius
    }
    
    override fn area() -> float {
        return 3.14159 * this.radius * this.radius
    }
    
    override fn perimeter() -> float {
        return 2 * 3.14159 * this.radius
    }
}
```

### 接口继承

```
trait Shape {
    fn area() -> float
}

trait Transformable extends Shape {
    fn scale(factor: float)
    fn translate(dx: float, dy: float)
}

class TransformableShape implements Transformable {
    // 必须实现 Shape 和 Transformable 的所有方法
}
```

## 设计模式

### 工厂模式

```
trait Animal {
    fn speak() -> str
}

class Dog implements Animal {
    fn speak() -> str { return "汪" }
}

class Cat implements Animal {
    fn speak() -> str { return "喵" }
}

fn create_animal(type: str) -> Animal {
    return match type {
        "dog" => Dog(),
        "cat" => Cat(),
        _ => Err("未知动物类型")
    }
}
```

### 观察者模式

```
trait Observer {
    fn update(message: str)
}

class EventManager {
    var listeners: [Observer]
    
    fn subscribe(listener: Observer) {
        this.listeners.push(listener)
    }
    
    fn notify(message: str) {
        for listener in this.listeners {
            listener.update(message)
        }
    }
}
```

## 最佳实践

1. **单一职责**：每个类应该只有一个职责
2. **封装**：隐藏内部实现细节
3. **组合优于继承**：优先使用组合而非深层继承
4. **接口隔离**：接口应该小而专注
5. **多态**：通过 trait 实现多态行为

## 下一步

- [标准库](/guide/stdlib) - 内置模块
- [FFI 接入](/guide/ffi) - 各语言接入指南
- [IoT 部署](/guide/iot) - 嵌入式编译与部署
