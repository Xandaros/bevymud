use bevy::prelude::*;

#[derive(Debug)]
pub enum Race {
    Human,
    Elf,
    Orc,
    Pixie,
}

impl ToString for Race {
    fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}

impl Race {
    pub fn as_str(&self) -> &'static str {
        match *self {
            Self::Human => "Human",
            Self::Elf => "Elf",
            Self::Orc => "Orc",
            Self::Pixie => "Pixie",
        }
    }
}

#[derive(Debug)]
pub enum Class {
    Warrior,
    Mage,
    Cleric,
}

impl ToString for Class {
    fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}

impl Class {
    pub fn as_str(&self) -> &'static str {
        match *self {
            Self::Warrior => "Warrior",
            Self::Mage => "Mage",
            Self::Cleric => "Cleric",
        }
    }
}
