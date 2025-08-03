package com.example.model;

import java.math.BigDecimal;
import java.time.LocalDateTime;
import java.util.List;

/**
 * Record representing an order in the system
 */
public record Order(String orderId, List<String> items, BigDecimal totalAmount, LocalDateTime createdAt) {
    
    /**
     * Creates an empty order with generated ID
     * @return a new empty Order instance
     */
    public static Order createEmpty() {
        return new Order(
            "ORDER-" + System.currentTimeMillis(),
            List.of(),
            BigDecimal.ZERO,
            LocalDateTime.now()
        );
    }
    
    /**
     * Creates an order from a list of items with calculated total
     * @param items the list of items
     * @param itemPrice price per item
     * @return a new Order instance
     */
    public static Order fromItems(List<String> items, BigDecimal itemPrice) {
        BigDecimal total = itemPrice.multiply(BigDecimal.valueOf(items.size()));
        return new Order(
            "ORDER-" + System.currentTimeMillis(),
            items,
            total,
            LocalDateTime.now()
        );
    }
    
    /**
     * Checks if the order is considered a large order
     * @param order the order to check
     * @return true if total amount is over 1000
     */
    public static boolean isLargeOrder(Order order) {
        return order.totalAmount.compareTo(BigDecimal.valueOf(1000)) > 0;
    }
    
    /**
     * Instance method to get item count
     * @return number of items in the order
     */
    public int getItemCount() {
        return items.size();
    }
}