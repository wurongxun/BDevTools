let _ = {
    isNullAndEmpty: function (obj: string | null) {
    if (typeof obj == "undefined" || obj == null || obj == "") {
        return true;
    } else {
        return false;
    }
},
isNotNullAndEmpty: function (obj: string | null) {
    if (typeof obj == "undefined" || obj == null || obj == "") {
        return false;
    } else {
        return true;
    }
}
}


export default _;