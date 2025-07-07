class DataManager {
    // This method has "search" in the declaration - should get higher score
    async searchUsers(searchTerm) {
        const users = [];
        for (let user of this.users) {
            // This "search" is in the body - should get lower score  
            if (user.name.includes(searchTerm)) {
                users.push(user);
            }
        }
        return users;
    }

    // This method has "search" only in the body - should get lower score
    async findData() {
        const data = await this.database.search("SELECT * FROM table");
        return data;
    }
}