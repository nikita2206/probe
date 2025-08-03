package com.example.model;

import java.time.LocalDate;
import java.time.Period;

/**
 * A record representing a person with basic information
 */
public record Person(String name, int age, String email) {
    
    /**
     * Creates a person with default values
     * @param name the person's name
     * @return a new Person instance with default age and email
     */
    public static Person withDefaults(String name) {
        return new Person(name, 0, "unknown@example.com");
    }
    
    /**
     * Creates an adult person (age 18+)
     * @param name the person's name
     * @param email the person's email
     * @return a new Person instance with age 18
     */
    public static Person createAdult(String name, String email) {
        return new Person(name, 18, email);
    }
    
    /**
     * Validates if the person's age is within reasonable bounds
     * @param person the person to validate
     * @return true if age is between 0 and 150, false otherwise
     */
    public static boolean isValidAge(Person person) {
        return person.age >= 0 && person.age <= 150;
    }
    
    /**
     * Instance method to check if person is an adult
     * @return true if age is 18 or over
     */
    public boolean isAdult() {
        return age >= 18;
    }
    
    /**
     * Gets a formatted display name
     * @return formatted name with age
     */
    public String getDisplayName() {
        return String.format("%s (%d years old)", name, age);
    }
}