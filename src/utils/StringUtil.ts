let _ = {
  isNullOrEmpty: function (obj: string | null) {
    if (typeof obj == "undefined" || obj == null || obj == "") {
      return true;
    } else {
      return false;
    }
  },
};

export default _;
