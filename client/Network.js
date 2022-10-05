/**
 * Namespace responsible for managing connection to server
 */
const Network = {};

/**
 * Get data from server
 */
Network.get = function get(url) {
	return new Promise(function(resolve, reject) {
		var req = new XMLHttpRequest();
		req.open('GET', url);

		req.onload = function() {
			// check if successful
			if (req.status == 200) {
				resolve(req.response);
			} else {
				reject(new Error(req.statusText+": "+req.response));
			}
		};
		req.onerror = function() {
			reject(Error("Network Error"));
		};
		req.send();
	});
};


/**
 * Post data to server
 */
Network.post = function (url, data) {
	return new Promise(function(resolve, reject) {
		var req = new XMLHttpRequest();
		req.open('POST', url);
		req.setRequestHeader("Content-type", "application/json");

		req.onload = function() {
			// check if successful
			if (req.status == 200) {
				resolve(req.response);
			} else {
				reject(new Error(req.statusText+": "+req.response));
			}
		};
		req.onerror = function() {
			reject(Error("Network Error"));
		};
		// server thinks it's a string data if you do this
		//req.send(JSON.stringify(body));
		req.send(data);
	});
};
function downloadData(filename, data) {
	var element = document.createElement('a');
	element.setAttribute('href', 'data:text/plain;charset=utf-8,' + encodeURIComponent(data));
	element.setAttribute('download', filename);
	element.style.display = 'none';
	document.body.appendChild(element);
	element.click();
	document.body.removeChild(element);
}
