package com.example.service;

/**
 * Service interface for user operations
 */
public interface UserService {
    
    /**
     * Retrieves a user by their ID
     * @param userId the unique identifier for the user
     * @return the user object if found, null otherwise
     */
    User getUserById(String userId);
    
    /**
     * Creates a new user account
     * @param username the desired username
     * @param email the user's email address
     * @return the created user object
     */
    User createUser(String username, String email);
    
    /**
     * Updates an existing user's information
     * @param userId the user's unique identifier
     * @param userDetails the new user information
     * @return true if update was successful, false otherwise
     */
    boolean updateUser(String userId, UserDetails userDetails);
    
    /**
     * Deletes a user account
     * @param userId the unique identifier of the user to delete
     * @return true if deletion was successful, false otherwise
     */
    boolean deleteUser(String userId);
    
    /**
     * Finds users by their email domain
     * @param emailDomain the email domain to search for
     * @return list of users with matching email domain
     */
    List<User> findUsersByEmailDomain(String emailDomain);
}