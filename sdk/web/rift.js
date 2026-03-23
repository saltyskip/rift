(function() {
  var DEFAULT_BASE = "https://api.riftl.ink";

  function detectPlatform() {
    if (typeof navigator === "undefined") return "other";
    var ua = navigator.userAgent.toLowerCase();
    if (/iphone|ipad|ipod/.test(ua)) return "ios";
    if (/android/.test(ua)) return "android";
    return "other";
  }

  var Rift = {
    open: function(linkId, opts) {
      if (typeof window === "undefined") return;
      opts = opts || {};
      var base = opts.baseUrl || DEFAULT_BASE;
      var platform = detectPlatform();

      var domain = opts.domain || (typeof location !== "undefined" ? location.hostname : undefined);

      var payload = { link_id: linkId };
      if (domain) payload.domain = domain;

      fetch(base + "/v1/sdk/click", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload)
      })
      .then(function(r) { return r.json(); })
      .then(function(data) {
        if (platform === "ios" && data.token && navigator.clipboard) {
          navigator.clipboard.writeText("rift:" + data.token).catch(function(){});
        }

        var deepLink = platform === "ios" ? data.ios_deep_link
                     : platform === "android" ? data.android_deep_link
                     : null;
        var storeUrl = platform === "ios" ? data.ios_store_url
                     : platform === "android" ? data.android_store_url
                     : null;

        if (platform === "android" && storeUrl && data.token) {
          var sep = storeUrl.indexOf("?") >= 0 ? "&" : "?";
          storeUrl += sep + "referrer=" + encodeURIComponent("rift_token=" + data.token);
        }

        if (deepLink) {
          window.location.href = deepLink;
          if (storeUrl) {
            setTimeout(function() { window.location.href = storeUrl; }, 1500);
          }
        } else if (storeUrl) {
          window.location.href = storeUrl;
        } else if (data.web_url) {
          window.location.href = data.web_url;
        }

        if (opts.onComplete) opts.onComplete(data);
      })
      .catch(function(err) {
        if (opts.onError) opts.onError(err);
      });
    },

    getLink: function(linkId, opts) {
      opts = opts || {};
      var base = opts.baseUrl || DEFAULT_BASE;
      return fetch(base + "/r/" + encodeURIComponent(linkId), {
        headers: { "Accept": "application/json" }
      }).then(function(r) { return r.json(); });
    }
  };

  if (typeof window !== "undefined") window.Rift = Rift;
  if (typeof module !== "undefined") module.exports = Rift;
})();
