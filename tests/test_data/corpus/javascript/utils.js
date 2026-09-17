function capitalizeString(str) {
    return str.charAt(0).toUpperCase() + str.slice(1);
}

function truncateString(str, maxLength) {
    if (str.length <= maxLength) {
        return str;
    }
    return str.substring(0, maxLength) + "...";
}

module.exports = {
    capitalizeString,
    truncateString,
};
