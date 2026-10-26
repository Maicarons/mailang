# Object-Oriented Programming

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang)

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

## Generic classes (current limitation: erased fields)

Generic functions monomorphize fully (`id::<int>` produces a dedicated
`id$int`), but **generic classes use an erased field layout**: a
`class Box<T>` stores its fields in the uniform `Value` layout at runtime and
no per-`T` class is generated. In practice:

```mai
class Box<T> {
    let value
    fn init(v) { self.value = v }
    fn get() { return self.value }
}

let b: Box<int> = Box(42)      // works: compiled to the erased layout
println(b.get())                 // 42
```

**Not provided** (this version):
- no per-type field layout monomorphization
- no full generic checking across fields (field access goes through `Value`)
- type parameters serve mainly as **annotations and documentation**

This limitation is recorded honestly in the ROADMAP (M3/L8) and the paper;
until field monomorphization is scheduled, "erased layout" is the documented
behaviour.

## Next Steps

- [Standard Library](/v0.4/en/guide/stdlib) - Built-in modules
- [FFI](/v0.4/en/guide/ffi) - Language integration
