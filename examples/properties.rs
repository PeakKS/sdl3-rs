extern crate sdl3;
use sdl3::properties::*;

#[derive(Debug, Clone)]
struct TestData {
    pub hello: String,
    pub goodbye: String,
}

impl Drop for TestData {
    fn drop(&mut self) {
        println!("TestData dropped: {self:?}");
    }
}

pub fn main() -> Result<(), String> {
    sdl3::init().ok();

    let mut properties = Properties::new().unwrap();

    let bprop = "bool";
    let fprop = "float";
    let nprop = "number";
    let sprop = "string";
    properties.set(bprop, true).ok();
    properties.set(fprop, 6.9).ok();
    properties.set(nprop, 420).ok();
    properties.set(sprop, "blazeit").ok();

    println!(
        "Property {bprop}: {:?}",
        properties.get(bprop, None::<bool>)
    );
    if let Ok(()) = properties.clear(bprop) {
        println!("Cleared {bprop}");
    } else {
        println!("Failed to clear {bprop}");
    }
    if let Ok(()) = properties.clear(bprop) {
        println!("Cleared {bprop}");
    } else {
        println!("Failed to clear {bprop}");
    }
    println!(
        "Property {bprop}: {:?}",
        properties.get(bprop, None::<bool>)
    );
    println!("Property {fprop}: {:?}", properties.get(fprop, None::<f32>));
    println!("Property {nprop}: {:?}", properties.get(nprop, None::<i64>));
    println!(
        "Property nodefault: {:?}",
        properties.get("nodefault", None::<i64>)
    );
    println!(
        "Property nodefault: {:?}",
        properties.get("nodefault", Some(42))
    );

    println!(
        "Property {sprop}: {:?}",
        properties.get(sprop, None::<String>)
    );

    let test = TestData {
        hello: String::from("hello"),
        goodbye: String::from("goodbye"),
    };

    // You can set a pointer by yourself, but you have to clean it up
    properties
        .set("pointer", Box::into_raw(Box::new(test.clone())))
        .ok();

    // Will get a pointer to the data
    if let Ok(pointer) = properties.get("pointer", None::<*mut TestData>) {
        unsafe {
            println!("Pointer: {:?}", *pointer);
        }
    } else {
        println!("Failed to get pointer");
    }

    // Will get a pointer to the data, then claim ownership of it and clear it from the properties object
    // You must clear a pointer property if you claim it
    if let Ok(pointer) = properties.get("pointer", None::<*mut TestData>) {
        unsafe {
            properties.clear("pointer").ok();
            let value = Box::from_raw(pointer);
            println!("Pointer from box: {:?}", value);
        }
    } else {
        println!("Failed to get pointer");
    }

    // The previous get cleared the property, so this will safely fail
    if let Ok(pointer) = properties.get("pointer", None::<*mut TestData>) {
        unsafe {
            println!("Pointer: {:?}", *pointer);
        }
    } else {
        println!("Failed to get pointer");
    }

    // Alternatively to setting a raw pointer, you can set a box which will be automatically cleaned up
    properties.set("autopointer", Box::new(test.clone())).ok();
    // Will get a pointer to the data
    if let Ok(pointer) = properties.get("autopointer", None::<*mut TestData>) {
        unsafe {
            println!("autopointer: {:?}", *pointer);
        }
    } else {
        println!("Failed to get autopointer");
    }
    // The box will be reclaimed and dropped at this point
    properties.clear("autopointer").ok();
    // This will fail
    if let Ok(pointer) = properties.get("autopointer", None::<*mut TestData>) {
        unsafe {
            println!("autopointer: {:?}", *pointer);
        }
    } else {
        println!("Failed to get autopointer");
    }

    // Set the autopointer again
    properties.set("autopointer", Box::new(test.clone())).ok();
    // semi-safely borrow a pointer property by holding a lock on properties
    properties
        .with("autopointer", |value: &TestData| {
            println!("Borrowed value: {value:?}");
        })
        .ok();
    // Overwrite the property, this will drop the previous value
    properties.set("autopointer", Box::new(test)).ok();

    properties
        .enumerate(Box::new(|properties, name| match name {
            Ok(name) => {
                print!("Enumeration: {name} ");
                if let Ok(thistype) = properties.get_type(name) {
                    match thistype {
                        PropertyType::BOOLEAN => println!("boolean"),
                        PropertyType::FLOAT => println!("float"),
                        PropertyType::NUMBER => println!("number"),
                        PropertyType::STRING => println!("string"),
                        PropertyType::POINTER => println!("pointer"),
                        _ => println!("invalid"),
                    }
                }
            }
            Err(error) => println!("Enumeration error: {error:?}"),
        }))
        .ok();

    Ok(())
}
