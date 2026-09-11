# Object-Oriented Programming

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [v0.1.0-oop](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-oop)

## Overview

MaìLang supports full OOP including classes, inheritance, traits, and polymorphism.

## Classes

### Basic Class

```
class Animal {
    let name: str
    let age: int

    fn init(name: str, age: int) {
        this.name = name
        this.age = age
    }

    fn speak() -> str {
        return "{this.name} speaks"
    }
}

let cat = Animal("Kitty", 3)
println(cat.speak())  // Kitty speaks
```

## Inheritance

```
class Dog extends Animal {
    let breed: str

    fn init(name: str, age: int, breed: str) {
        super(name, age)
        this.breed = breed
    }

    override fn speak() -> str {
        return "{this.name} barks!"
    }
}

let dog = Dog("Rex", 3, "Labrador")
println(dog.speak())  // Rex barks!
```

## Traits

```
trait Printable {
    fn to_string() -> str
    fn print() {
        println(this.to_string())
    }
}

class Point implements Printable {
    let x: float
    let y: float

    fn to_string() -> str {
        return "({this.x}, {this.y})"
    }
}

let p = Point(3.0, 4.0)
p.print()  // (3, 4)
```

## Next Steps

- [Standard Library](/en/guide/stdlib) - Built-in modules
- [FFI](/en/guide/ffi) - Language integration
