
var saveCount = 0;
/* constants related to rendering graph */

var Colors = {
	action: "#0000FF",
	energized: "#24B700",
	damaged: "#c70039",
	shadow: "#574f7d",
	risky: "#FDDC01",
}

const lineWeight = 6;
const hoveredLineWeight = 8;
const nodeRadius = 9;
const nodeWeight = 2;


const riskThreshold = 0.8;
const blinkTime = 500;

const antPathDashArray = [2, 40];
const antPathDelay = 700;

var Icons = {};

const IconSize = [50, 50];
const IconAnchor = [25, 25];

Icons.tower = L.divIcon({
	className: 'divIcon',
	html: "<div class='markerDiv'><img src='assets/tower.png' /></div>",
	iconSize: IconSize,
	iconAnchor: IconAnchor
});

Icons.solar = L.divIcon({
	className: 'divIcon',
	html: "<div class='markerDiv'><img src='assets/solar-panel.png' /></div>",
	iconSize: IconSize,
	iconAnchor: IconAnchor
});

Icons.crosshair = L.icon({
    iconUrl: 'assets/target.png',
    iconSize:     IconSize, // size of the icon
    iconAnchor:   IconAnchor, // point of the icon which will correspond to marker's location
});


// graph helper functions

function createEnergizedAntPath(route, nodeStatus=1) {
	return L.polyline.antPath(route, {
		delay: antPathDelay,
		dashArray: antPathDashArray,
		color: nodeStatus == 2 ? Colors.action : Colors.energized,
		pulseColor: "#FFFFFF",
		weight: lineWeight,
		smoothFactor: 1,
		paused: false,
		reversed: false,
		"hardwareAccelerated": true,
		pane: "branches"
	});
}

function createEnergizedArrow(route, nodeStatus=1) {
	return L.polyline(route, {
		color: nodeStatus == 2 ? Colors.action : Colors.energized,
		weight: lineWeight,
		smoothFactor: 1,
		pane: "nodes"
	});
}

function createArrowDecorator(line, nodeStatus=1) {
	return L.polylineDecorator( line, {
		patterns: [
			//{offset: '100%', repeat: 0, symbol: L.Symbol.arrowHead({pixelSize: 15, polygon: false, pathOptions: {stroke: true}})}
			{offset: '50%', repeat: 0, symbol: L.Symbol.arrowHead({
				pixelSize: 25, pathOptions: {
					color: nodeStatus == 2 ? Colors.action : Colors.energized,
					fillOpacity: 1, weight: 0
				},
			})}

		]
	});
}

function getNodeMarker(node, i) {
		return L.marker(node.latlng, {
			icon: L.divIcon({
				className: 'divIcon',
				html: `<div style="border-radius: 100%; width: 100%; height: 100%; background: white; color: black; font-family: serif; font-size: 1.5em; border: 2px solid black; text-align: center; display: flex; align-content: center; justify-content: center;">${i+1}</div>`,
				iconSize: [30,30],
				iconAnchor: [15,15]
			}),
			pane: "nodes"
		});
}

var NodeNaming = {
	name: function(node) {
		return node.name ? node.name : "#"+node.index;
	},
	index: function(node) {
		return "#"+node.index;
	},
	index1: function(node) {
		return "#"+(node.index+1);
	},
};
function getNodeName(node) {
	return NodeNaming[Settings.nodeName](node);
}


class Graph {
	constructor(map, options) {
		this.map = map;
		/**
		 * 0 = normal mode
		 * 1 = edit mode
		 */
		this.mode = 0;
		this.lastHover = { type: null, data: null, hovered: false };
		this.lastEdit = { type: null, data: null, active: false };
		/**
		 * Contains objects of form 
		 * { latlng = [lat, lng] }
		 * where lat and lng are the latitude and longtitude of the node.
		 * A property called "branches" will be used to store the branches that
		 * are connected to this node.
		 */
		this.nodes = [];
		/**
		 * Contains objects of form
		 * { nodes: [i_0, i_1], status: s, source = L }
		 * where i_0 and i_1 are the indexes of nodes that the branch connects;
		 * s is 0 for unknown, -1 for damaged, 1 for energized
		 * L is the index of the DER that powers the branch if the branch is energized
		 */
		this.branches = [];
		/**
		 * Contains objects of form
		 * { node: i, status: s, source = L }
		 * similar to branches, but connects a resource and node
		 */
		this.externalBranches = [];
		/**
		 * Includes transmission grid and DERs
		 * { latlng: [lat, lng], type = type }
		 * where type is a string describing the type of DER or null for transmission grid
		 */
		this.resources = [];
		this.requiredFields = ["name", "nodes", "branches", "externalBranches", "resources", "view", "zoom"];
		// this field becomes true if the user enters edit mode
		// (modifies the graph)
		this.dirty = false;
		this.name = "unnamed"
		Object.assign(this, options);
		this.setEventHandlers();
	}
	/**
	 * Loads the graph data, doesn't render
	 */
	loadGraph(g) {
		this.requiredFields.forEach(field => {
			if(g[field]) {
				this[field] = g[field];
			} else {
				console.log("Warning: field not found in graph:", field);
			}
		});
		if(g.solutionFile) this.solutionFile = g.solutionFile;
		// nodes need to know their indexes
		this.nodes.forEach((node, i) => {
			node.index = i;
		});
		BottomRightPanel.classList.add("hidden");
		if(g.view && g.zoom) {
			this.map.flyTo(g.view, g.zoom);
		}
		this.dirty = false;
		this.rerender();
	}
	readFile(event) {
		var file = event.target.files[0];
		if (!file) {
			return;
		}
		var reader = new FileReader();
		reader.onload = function(e) {
			var contents = e.target.result;
			let parsed;
			try {
				parsed = JSON.parse(contents);
			} catch(error) {
				let div = d3.create("div").style("margin", "40vh 0");
				div.append("h1").text("Error while reading file");
				div.append("p").style("font-family", "monospace")
					.text(error.message);
				showModalOverlay(div.node(), { warning: true });
				return;
			}
			if(parsed.nodes && parsed.branches && parsed.externalBranches
				&& parsed.resources) {
				// TODO: more checks
				this.loadGraph(parsed);
				this.rerender();
			} else {
				let div = d3.create("div");
				div.append("h1").text("Malformed File");
				div.append("p").text("Given JSON lacks at least one of the following fields:");
				div.append("ul").selectAll("li")
					.data(this.requiredFields).join("li")
					.text(d => d);
				showModalOverlay(div.node(), { warning: true });
			}
		};
		reader.readAsText(file);
	}
	/**
	 * Converts the graph into a serialized object, ready to stringify
	 */
	serialize() {
		let g = {};
		g.name = this.name;
		g.nodes = this.nodes.map(n => {
			return {
				latlng: n.latlng,
				pf: n.pf,
				name: n.name,
				status: n.status ? n.status : 0
			};
		});
		g.branches = this.branches.map(b => {
			return {
				nodes: b.nodes,
			};
		});
		g.externalBranches = this.externalBranches;
		g.resources = this.resources;
		g.view = this.map.getCenter();
		g.zoom = this.map.getZoom();
		return g;
	}
	/**
	 * Returns the JSON.stringified version of the serialized graph
	 * Indented.
	 */
	getJson() {
		return JSON.stringify(this.serialize(), null, 4);
	}
	saveFile(filename=null) {
		if(!filename) filename = "graph"+saveCount+".json";
		downloadData(filename, this.getJson());
		saveCount++;
	}

	/**
	 * Event handlers
	 */
	setEventHandlers(mode=0) {
		if(mode == 0) {
			this.nodeOnClick = this.nodeOnInfo;
			this.branchOnClick = this.branchOnInfo;
			this.resourceOnClick = this.resourceOnInfo;
		} else {
			this.nodeOnClick = this.nodeOnEdit;
			this.branchOnClick = this.branchOnEdit;
			this.resourceOnClick = this.resourceOnEdit;
		}
	}
	nodeOnMouseOver (event) {
		this.lastHover.type = "node";
		this.lastHover.data = event.target.node;
		this.lastHover.pos = event.target._latlng;
		this.lastHover.hovered = true;
		event.target.setRadius(nodeRadius*1.25);
		let node = event.target.node;
		let pf = node.pf;
		let status;
		switch(node.status){
			case -1: status="Damaged"; break;
			case 1: status="Energized"; break;
			default: status="Unknown"; break;
		}
		// Lat: ${Math.round(10000*event.target._latlng.lat)/10000} <br/>
		// Lng: ${Math.round(10000*event.target._latlng.lng)/10000}
		Tooltip.div.innerHTML =
			`<b>${getNodeName(node)}</b> <br/>
			  Status: ${status} <br/>
			P<sub>f</sub>: ${pf != null ? pf.toFixed(3) : "Unknown"}
		`;
		Tooltip.show(event.originalEvent);
	}
	nodeOnMouseOut (event) {
		this.lastHover.hovered = false;
		event.target.setRadius(nodeRadius);
		Tooltip.hide();
	}
	showNodeInfo(node) {
		let pf = node.pf;
		let status;
		switch(node.status){
			case -1: status="Damaged"; break;
			case 1: status="Energized"; break;
			default: status="Unknown"; break;
		}
		BottomRightPanel.show({
			node: node,
		});
		BottomRightPanelContent.innerHTML = `
			<h1>${getNodeName(node)}</h1>
			<p>Lat: ${Math.round(10000*node.latlng[0])/10000}</p>
			<p>Lng: ${Math.round(10000*node.latlng[1])/10000}</p>
			<p>Status: ${status}</p>
			${node.id ? "<p>ID: "+node.id+"</p>" : ""}
			${node.addr ? "<p>Address: "+node.addr+"</p>" : ""}
			<p>Probability of Failure: ${pf ? Math.round(10000*pf)/10000 : "Unknown"}</p>
			<p>Connected to ${node.branches.length} branches.</p>
			`;
		if(policyView && policyView.nodeOnInfo) {
			policyView.nodeOnInfo(node, BottomRightPanelContent);
		}
		if(node.externalBranches.length>0) {
			BottomRightPanelContent.innerHTML += "<p>Connected to a resource.</p>";
		}
	}
	nodeOnUpdate(node) {
		let pf = node.pf;
		let status;
		switch(node.status){
			case -1: status="Damaged"; break;
			case 1: status="Energized"; break;
			default: status="Unknown"; break;
		}
		BottomRightPanel.show({
			node: node,
		});
		BottomRightPanelContent.innerHTML = `
			<h1>${getNodeName(node)}</h1>
			<p>Lat: ${Math.round(10000*node.latlng[0])/10000}</p>
			<p>Lng: ${Math.round(10000*node.latlng[1])/10000}</p>
			<p>Status: ${status}</p>
			${node.id ? "<p>ID: "+node.id+"</p>" : ""}
			${node.addr ? "<p>Address: "+node.addr+"</p>" : ""}
			<p>Probability of Failure: 
			${pf == null ? "Unknown" : Math.round(10000*pf)/10000}</p>
			<p>Connected to ${node.branches.length} branches.</p>
			`;
		if(policyView && policyView.nodeOnInfo) {
			policyView.nodeOnInfo(node, BottomRightPanelContent);
		}
		if(node.externalBranches.length>0) {
			BottomRightPanelContent.innerHTML += "<p>Connected to a resource.</p>";
		}
		let div = d3.select(BottomRightPanelContent);
		div.append("div").classed("blockButton", true)
			.text("Set probability of failure = 0")
			.on("click", () => {
				if(!("originalPf" in node)) node.originalPf = node.pf;
				node.pf = 0.0;
				this.nodeOnUpdate(node);
			});
		div.append("div").classed("blockButton", true)
			.text("Set probability of failure = 1")
			.on("click", () => {
				if(!("originalPf" in node)) node.originalPf = node.pf;
				node.pf = 1.0;
				this.nodeOnUpdate(node);
			});
	}
	nodeOnInfo(event) {
		let node = event.target.node;
		if(node) this.showNodeInfo(node);
	}
	nodeOnEdit(event) {
		let node = event.target.node;
		this.lastEdit.type = "node";
		this.lastEdit.data = node;
		this.lastEdit.active = true;
		BottomRightPanel.show();
		BottomRightPanelContent.innerHTML = `
			<h1>Node #${this.nodes.indexOf(node)}</h1>
			<p>Connected to ${node.branches.length} branches.</p>
			`;
		let controls = d3.create("div");
		let nameInput = createTextInput(controls, "Name", node.name);
		let latInput = createTextInput(controls, "Lat", node.latlng[0]);
		let lngInput = createTextInput(controls, "Lng", node.latlng[1]);
		let status = createSelectBox(controls,[
			{name: "Damaged", value: -1},
			{name: "Unknown", value: 0},
			{name: "Energized", value: 1},
		], "status", node.status ? node.status : 0);
		let pfInput = createTextInput(controls, "Probability of Failure", 
			node.pf ? node.pf : "Unknown");
		controls.append("div").classed("blockButton", true)
			.text("OK")
			.on("click", () => {
				let lat = parseFloat(latInput.property("value"));
				let lng = parseFloat(lngInput.property("value"));
				node.latlng = [
					isNaN(lat) ? node.latlng[0] : lat,
					isNaN(lng) ? node.latlng[1] : lng,
				];
				latInput.property("value", node.latlng[0]);
				lngInput.property("value", node.latlng[1]);
				let pf = parseFloat(pfInput.property("value"));
				if(isNaN(pf)) {
					node.pf = null;
					pfInput.property("value", "Unknown");
				} else {
					node.pf = Math.min(1.0, Math.max(0.0, pf));
					pfInput.property("value", node.pf);
				}
				let newName = nameInput.property("value");
				if(newName.length > 0) node.name = newName;
				else delete node.name;
				node.status = status.value;
				this.rerender();
			});
		BottomRightPanelContent.appendChild(controls.node());
	}
	resourceOnMouseOver(event) {
		this.lastHover.type = "resource";
		this.lastHover.data = event.target.data;
		this.lastHover.pos = event.target._latlng;
		this.lastHover.hovered = true;
		let resource = event.target.data;
		let name = resource.type ? "DER" : "Transmission Grid";
		Tooltip.div.innerHTML =
			`${name} <br/>
			${resource.type ? "Type: "+resource.type+"<br/>" : ""}
	  Lat: ${Math.round(10000*event.target._latlng.lat)/10000} <br/>
	  Lng: ${Math.round(10000*event.target._latlng.lng)/10000}`
		Tooltip.show(event.originalEvent);
	}
	resourceOnInfo(event) {
		let resource = event.target.data;
		let name = resource.type ? "DER" : "Transmission Grid";
		BottomRightPanel.show();
		BottomRightPanelContent.innerHTML = `
			<h1>${name}</h1>
			${resource.type ? "<p>Type: "+resource.type+"</p>" : ""}
			<p>Lat: ${Math.round(10000*resource.latlng[0])/10000}</p>
			<p>Lng: ${Math.round(10000*resource.latlng[1])/10000}</p>
			`;
	}
	resourceOnEdit(event) {
		let resource = event.target.data;
		this.lastEdit.type = "resource";
		this.lastEdit.data = resource;
		this.lastEdit.active = true;
		BottomRightPanel.show();
		BottomRightPanelContent.innerHTML = `
			<h1>Resource #${this.resources.indexOf(resource)}</h1>
			`;
		let controls = d3.create("div");
		let latInput = createTextInput(controls, "Lat", resource.latlng[0]);
		let lngInput = createTextInput(controls, "Lng", resource.latlng[1]);
		let type = createSelectBox(controls,[
			{name: "Transmission Grid", value: null},
			{name: "Solar Panel", value: "solar"},
		], "Type", resource.type);
		controls.append("div").classed("blockButton", true)
			.text("OK")
			.on("click", () => {
				let lat = parseFloat(latInput.property("value"));
				let lng = parseFloat(lngInput.property("value"));
				resource.latlng = [
					isNaN(lat) ? resource.latlng[0] : lat,
					isNaN(lng) ? resource.latlng[1] : lng,
				];
				resource.type = type.value;
				latInput.property("value", resource.latlng[0]);
				lngInput.property("value", resource.latlng[1]);
				this.rerender();
			});
		BottomRightPanelContent.appendChild(controls.node());
	}
	resourceOnMouseOut(event) {
		this.lastHover.hovered = false;
		Tooltip.hide();
	}
	branchOnMouseOver (event) {
		this.lastHover.type = "branch";
		this.lastHover.data = event.target.branch;
		this.lastHover.pos = event.target._latlng;
		this.lastHover.hovered = true;
		event.target.setStyle({ weight: hoveredLineWeight });
		/*
		let status;
		switch(event.target.branch.status){
			case -1: status="Damaged"; break;
			case 1: status="Energized"; break;
			default: status="Unknown"; break;
		}
		*/
		Tooltip.div.innerHTML =
			`Branch #${this.branches.indexOf(event.target.branch)} <br/>
		`;
		Tooltip.show(event.originalEvent);
	}
	branchOnInfo(event) {
		let branch = event.target.branch;
		BottomRightPanel.show();
		BottomRightPanelContent.innerHTML =
			`<h1>Branch #${this.branches.indexOf(branch)} </h1>
			  <p>Connects nodes ${branch.nodes[0]} and ${branch.nodes[1]}</p>
			`;
	}
	branchOnEdit(event) {
		let branch = event.target.branch;
		BottomRightPanel.show();
		BottomRightPanelContent.innerHTML =
			`<h1>Branch #${this.branches.indexOf(branch)} </h1>
			  <p>Connects nodes ${branch.nodes[0]} and ${branch.nodes[1]}</p>
			`;
		let controls = d3.create("div");
		controls.append("div").classed("blockButton", true)
			.text("Reverse")
			.on("click", () => {
				branch.nodes.reverse();
				this.rerender();
			});
		BottomRightPanelContent.appendChild(controls.node());
	}
	branchOnMouseOut (event) {
		this.lastHover.hovered = false;
		event.target.setStyle({ weight: lineWeight });
		Tooltip.hide();
	}
	externalBranchOnMouseOver (event) {
		this.lastHover.type = "externalBranch";
		this.lastHover.data = event.target.branch;
		this.lastHover.pos = event.target._latlng;
		this.lastHover.hovered = true;
		event.target.setStyle({ weight: hoveredLineWeight });
		let status;
		switch(event.target.branch.status){
			case -1: status="Damaged"; break;
			case 1: status="Energized"; break;
			default: status="Unknown"; break;
		}
		Tooltip.div.innerHTML =
			`External Branch #${this.externalBranches.indexOf(event.target.branch)} <br/>
	  Status: ${status}`
		Tooltip.show(event.originalEvent);
	}
	externalBranchOnMouseOut (event) {
		this.lastHover.hovered = false;
		event.target.setStyle({ weight: lineWeight });
		Tooltip.hide();
	}


	/**
	 * Renders the this.nodes into new layers
	 */
	render (map) {
		this.nodes.forEach(node => {
			// holds the branch elements displayed on map
			node.branches = [];
			// holds the branch elements that connect to DERs
			node.externalBranches = [];
		});
		this.riskyNodes = [];
		if(this.blinkTimer) clearTimeout(this.blinkTimer);
		var markers = [];
		var circles = [];
		var branches = [];
		var externalBranches = [];
		var resourceMarkers = [];
		var decorators = [];
		var nodeInfos = [];
		const branchMode = this.mode == 0 ? "branches" : "nodes";
		let pfInterpolator;
		if(Settings.colorized) 
			pfInterpolator = d3.interpolateRgb(Colors.shadow, Colors.risky);
		else
			pfInterpolator = (_) => Colors.shadow;

		// add branches
		for(var i = 0; i<this.branches.length; ++i) {
			let branch = this.branches[i];
			let route = branch.nodes.map(j => this.nodes[j].latlng);
			let line;
			let energized = branch.energized;
			if (!Settings.renderNextState && energized == 2) energized = 0;
				//this.nodes[branch.nodes[0]].status > 0 &&
				//	this.nodes[branch.nodes[1]].status > 0;
			if (energized) {
				if(this.mode == 0 && Settings.animateAnts) {
					line = createEnergizedAntPath(route, energized);
				} else {
					line = createEnergizedArrow(route, energized);
				}
			} else {
				line = L.polyline(route, {
					color: Colors.shadow,
					weight: lineWeight,
					smoothFactor: 1,
					pane: branchMode
				});
			}
			if(Settings.arrows && energized) {
				decorators.push(createArrowDecorator(line, energized));
			}
			line.branch = branch;
			line.on("mouseover", this.branchOnMouseOver.bind(this));
			line.on("click", this.branchOnClick.bind(this));
			line.on("mouseout", this.branchOnMouseOut.bind(this));
			branches.push(line);
			this.nodes[branch.nodes[0]].branches.push(line);
			this.nodes[branch.nodes[1]].branches.push(line);
		}
		// add external branches
		for(var i = 0; i<this.externalBranches.length; ++i) {
			let branch = this.externalBranches[i];
			let lineColor = (branch.status>0) ? Colors.energized :
				(branch.status===0) ? Colors.shadow : Colors.damaged;
			let line = L.polyline( [ //coordinates
				this.resources[branch.source].latlng,
				this.nodes[branch.node].latlng
			], {
				color: lineColor,
				weight: lineWeight,
				smoothFactor: 1,
				pane: branchMode
			});
			// rename to externalBranch?
			line.branch = branch;
			line.on("mouseover", this.externalBranchOnMouseOver.bind(this));
			line.on("mouseout", this.externalBranchOnMouseOut.bind(this));
			externalBranches.push(line);
			this.nodes[branch.node].externalBranches.push(line);
		}
		// add nodes
		for(var i = 0; i<this.nodes.length; ++i) {
			let node = this.nodes[i];
			let latlng = node.latlng;
			let color = pfInterpolator(node[Settings.colorizationTarget]);
			if(node.status>0) {
				if(node.status == 2) {
					if(Settings.renderNextState) color = Colors.action;
				} else {
					color = Colors.energized;
				}
			} else if(node.status<0) {
				color = Colors.damaged;
			}

			let circle = L.circleMarker(latlng, {
				color: Colors.shadow,
				fillColor: color,
				fillOpacity: 1.0,
				radius: nodeRadius,
				weight: nodeWeight,
				pane: "nodes"
			});
			if(node.status == 0 &&
				 node[Settings.colorizationTarget] > riskThreshold) {
				circle.originalColor = color;
				this.riskyNodes.push(circle);
			}
			circle.node = node;
			circle.on("mouseover", this.nodeOnMouseOver.bind(this));
			circle.on("mouseout", this.nodeOnMouseOut.bind(this));
			circle.on("click", this.nodeOnClick.bind(this));
			circles.push(circle);
			if(Settings.renderNodeInfoOnMap) {
				let name = `<b>${getNodeName(node)}</b>`;
				let pfinfo = `<i>P<sub>f</sub></i>&nbsp; = ${node.pf}`;
				let text = name + "<br/>" + (Settings.nopf ? "" : pfinfo);
				let customStyle = Settings.nopf ? "padding: 0.1em;" : "";
				customStyle += node.customLabelStyle ? node.customLabelStyle : "";
				let m = L.marker(node.latlng, {
					className: "divIcon",
					icon: getNodeDescriptionIcon(text, "nodeInfoMarker", customStyle),
					pane: "nodeInfo",
				});
				nodeInfos.push(m);
			}
		}
		// add resources
		for(var i = 0; i<this.resources.length; ++i) {
			let resource = this.resources[i];
			let type = resource.type ? resource.type : "tower";
			let marker = L.marker(resource.latlng, {
				icon: Icons[type],
				pane: "resources"
			});
			marker.data = resource;
			marker.on("mouseover", this.resourceOnMouseOver.bind(this));
			marker.on("click", this.resourceOnClick.bind(this));
			marker.on("mouseout",  this.resourceOnMouseOut.bind(this));
			resourceMarkers.push(marker);
		}

		this.map = map;

		let allMarkers = [].concat(
			markers,
			resourceMarkers,
			circles,
			branches,
			externalBranches,
			decorators,
			nodeInfos
		);
		this.markerLayer = L.featureGroup(allMarkers);
		this.markerLayer.addTo(map);
		this.blinkTimer = setTimeout(() => this.onBlink(), blinkTime);
	}
	onBlink(even=true) {
		try {
			if(even) {
				for(let circle of this.riskyNodes)
					circle.setStyle({
						// color: Colors.damaged,
						fillColor: Colors.damaged,
					});
			} else {
				for(let circle of this.riskyNodes)
					circle.setStyle({
						// color: circle.originalColor,
						fillColor: circle.originalColor,
					});
			}
			this.blinkTimer = setTimeout(() => this.onBlink(!even), blinkTime);
		} catch(e) {
			console.error("onBlink failed ", e);
		}
	}
	/**
	 * Generates a random system with len nodes.
	 * For testing purposes.
	 */
	generateRandom (len) {
		const variety = 0.01;
		let lat = 41;
		let lng = 29.045;
		this.nodes = [];
		this.branches = [];
		for(var i =0; i<len; ++i) {
			lat += (Math.random()*variety)-(variety/2);
			lng += (Math.random()*variety)-(variety/2);
			let node = { latlng: [lat,lng] };
			if(i>0) {
				this.branches.push({
					nodes: [i-1, i],
					status: (Math.random() < 0.2) ? -1 : (Math.random() < 0.5) ? 0 : 1
				});
			}
			this.nodes.push(node);
		}
	}
	setMode(mode) {
		if(this.mode == mode) return;
		this.mode = mode;
		if(this.mode == 1) {
			resetPolicyScreen();
			this.dirty = true;
			TopLeftPanel.classList.add("warning");
			TopLeftPanel.innerText = "EDIT MODE";
			let topbar = d3.create("div").attr("id", "TopBar");
			topbar.append("label").classed("blockButton", true)
				.classed("alt", true)
				.attr("for", "fileInput")
				.text("Load");
			topbar.append("input").attr("id", "fileInput")
				.attr("type", "file")
				.node()
				.addEventListener("change", this.readFile.bind(this), false);

			topbar.append("div").classed("blockButton", true)
				.classed("alt", true)
				.text("Save")
				.on("click", () => {
					this.saveFile();
				});
			topbar.append("div").classed("blockButton", true)
				.classed("alt", true)
				.text("Clear")
				.on("click", () => {
					this.nodes = [];
					this.branches = [];
					this.externalBranches = [];
					this.resources = [];
					this.rerender();
				});
			topbar.style("transform", "translate(0, -53px)")
				.transition().duration(1000)
				.style("transform", "translate(0, 0)");
			this.topbar = topbar;
			HUD.appendChild(topbar.node());
			BottomRightPanel.show();
			BottomRightPanelContent.innerHTML =
			`<h1>Edit Mode</h1>
			  <p>a: add node</p>
			  <p>s: add branch</p>
			  <p>q: add transmission grid</p>
			  <p>w: add solar panel</p>
			`;
		} else {
			TopLeftPanel.classList.remove("warning");
			TopLeftPanel.innerText = "METU CPS";
			let topbar = this.topbar;
			topbar.transition().duration(1000)
				.style("transform", "translate(0, -53px)")
				.on("end", () => { topbar.remove(); });
			BottomRightPanel.classList.add("hidden");
		}
		this.setEventHandlers(mode);
		this.rerender();
	}
	addBranch(source, target) {
		let i0 = this.nodes.indexOf(source);
		let i1 = this.nodes.indexOf(target);
		this.branches.push({
			nodes: [i0, i1],
			status: 0,
		});
	}
	addExternalBranch(source, target) {
		let i0 = this.resources.indexOf(source);
		let i1 = this.nodes.indexOf(target);
		this.externalBranches.push({
			source: i0,
			node: i1,
			status: 1,
		});
	}
	addNode(position) {
		let newNode = {
			"latlng": position
		}
		this.nodes.push(newNode);
	}
	removeElement(victim) {
		let i;
		let data = victim.data;
		switch(victim.type) {
			case "node":
				i = this.nodes.indexOf(data);
				this.nodes.splice(i, 1);
				this.branches = this.branches.filter(b => {
					return !b.nodes.includes(i);
				});
				// decrease the indexes of following nodes
				this.branches.forEach((b) => {
					b.nodes = b.nodes.map(n =>
						n > i ? n-1 : n
					);
				});
				this.externalBranches = this.externalBranches.filter(b => b.node != i);
				this.externalBranches.forEach((b) => {
					if(b.node > i) b.node--;
				});
				return true;
			case "resource":
				i = this.resources.indexOf(data);
				this.resources.splice(i, 1);
				this.branches.forEach(b => {
					if(b.source === i) {
						if(b.status == 1) b.status = 0;
						b.source = null;
					} else if(b.source > i) {
						b.source--;
					}
				});
				this.externalBranches = this.externalBranches.filter(b => b.source !== i);
				this.externalBranches.forEach(b => {
					if(b.source === i) {
						if(b.status == 1) b.status = 0;
						b.source = null;
					} else if(b.source > i) {
						b.source--;
					}
				});
				return true;
			case "branch":
				i = this.branches.indexOf(data);
				this.branches.splice(i, 1);
				return true;
			case "externalBranch":
				i = this.externalBranches.indexOf(data);
				this.externalBranches.splice(i, 1);
				return true;
			default:
				return false;
		}
		
	}
	handleKeyDown(event) {
		if(this.mode == 1) {
			switch(event.key) {
				case "a":
					if(!this.map.mousePos) return;
					this.addNode(this.map.mousePos);
					this.rerender();
					break;
				case "s":
					if(!this.lastHover.hovered) {
						if(this.crosshair) this.crosshair.remove();
						this.lastSelect = null;
						return;
					}
					if (this.lastHover.type !== "node" && this.lastHover.type !== "resource") break;
					if(this.lastSelect) {
						if(this.lastSelect.data !== this.lastHover.data) {
							if(this.lastSelect.type === "node") {
								if(this.lastHover.type === "node") {
									this.addBranch(
										this.lastSelect.data,
										this.lastHover.data);
									this.rerender();
								} else if (this.lastHover.type === "resource") {
									this.addExternalBranch(
										this.lastHover.data,
										this.lastSelect.data);
									this.rerender();
								}
							} else if(this.lastSelect.type === "resource") {
								if(this.lastHover.type === "node") {
									this.addExternalBranch(
										this.lastSelect.data,
										this.lastHover.data);
									this.rerender();
								}
							}
						}
					}
					if(this.crosshair) this.crosshair.remove();
					this.crosshair = L.marker(this.lastHover.pos, {
						icon: Icons.crosshair
					}).addTo(this.map);
					this.crosshair._icon.style.pointerEvents = "none";
					this.lastSelect = Object.assign({}, this.lastHover);
					break;
				case "x":
					if(this.lastHover.hovered) {
						if(this.removeElement(this.lastHover)) {
							this.rerender();
						}
					}
					break;
				case "q":
					if(!this.map.mousePos) return;
					this.resources.push({
						latlng: this.map.mousePos,
						type: null,
					});
					this.rerender();
					break;
				case "w":
					if(!this.map.mousePos) return;
					this.resources.push({
						latlng: this.map.mousePos,
						type: "solar",
					});
					this.rerender();
					break;
				default:
					break;
			}
		}
	}
	clear() {
		if(this.markerLayer) {
			this.markerLayer.clearLayers();
			this.markerLayer.remove();
		}
	}
	/**
	 * Renders the graph. If it is already rendered, clears before rendering.
	 */
	rerender() {
		this.clear();
		this.render(this.map);
	}
	contextMenu(event) {
		if(this.mode == 0) this.normalContextMenu(event);
		else this.editContextMenu(event);
	}
	normalContextMenu(event) {
		let menu = d3.select("#ContextMenu").html("");
		if(this.contextMenuListener) {
			this.contextMenuListener(event, menu);
		}
		if(this.lastHover.hovered && this.lastHover.type == "node") {
			menu.append("div")
				.text("Update Bus")
				.on("click", () => {
					this.nodeOnUpdate(this.lastHover.data);
					policyView.updateMode();
				});
		}
		menu.append("div")
			.text("Menu")
			.on("click", onMenuButton);
	}
	editContextMenu(event) {
		let position = [event.latlng.lat, event.latlng.lng];
		let menu = d3.select("#ContextMenu").html("");
		if(this.lastHover.hovered) {
			menu.append("div")
				.text("Remove")
				.on("click", () => {
					if(this.removeElement(this.lastHover))
						this.rerender();
				});
		} else {
			if(this.lastEdit.active && 
				(this.lastEdit.type == "node" || 
					this.lastEdit.type == "resource"))
			{
				menu.append("div")
					.text("Move Here")
					.on("click", () => {
						this.lastEdit.data.latlng = position;
						this.rerender();
					});
			}
			menu.append("div")
				.text("Add Node")
				.on("click", () => {
					this.addNode(position);
					this.rerender();
				});
			menu.append("div")
				.text("Add Resource")
				.on("click", () => {
					this.resources.push({
						latlng: position,
						type: null,
					});
					this.rerender();
				});
			menu.append("div")
				.text("Add Solar Panel")
				.on("click", () => {
					this.resources.push({
						latlng: position,
						type: "solar",
					});
					this.rerender();
				});
		}
		menu.append("div")
			.text("Switch to Normal Mode")
			.on("click", () => {
				this.setMode(0);
			});
	}
	_setState(state) {
		if(state.length !== this.nodes.length) {
			throw new Error("State length is not equal to nodes.length "
				+this.nodes.length);
		}
		for(let i=0; i<state.length; ++i) {
			switch(state[i]) {
				case "D":
					this.nodes[i].status = -1;
					break;
				case "U":
					this.nodes[i].status = 0;
					break;
				default:
					if(this.nodes[i].status > 0)
						this.nodes[i].status = 1;
					else
						this.nodes[i].status = 2;
			}
		}
	}
	/**
	 * Traverse graph breadth-first, starting from each
	 * energy source (add enterance points to queue), figure out directions
	 * as you go.
	 */
	_computeBranchDirections() {
		let queue = [];
		let oldNodes = new Set(); // constant time lookup
		for(let externalBranch of this.externalBranches) {
			queue.push(externalBranch.node);
		}
		while (queue.length > 0) {
			let node = queue.shift();
			for(let b of this.nodes[node].branches) {
				if(oldNodes.has(b.branch.nodes[0])) continue;
				if(oldNodes.has(b.branch.nodes[1])) continue;
				if(b.branch.nodes[0] == node) {
					if(state[b.branch.nodes[1]] == state[node]) {
						if(state[node] != "D" && state[node] != "U" &&
								!queue.includes(b.branch.nodes[1])) {
							b.branch.energized = b.branch.energized > 0 ? 1 : 2;
							queue.push(b.branch.nodes[1]);
						}
					}
				} else if(state[b.branch.nodes[0]] == state[node]) {
					if(state[node] != "D" && state[node] != "U" &&
							!queue.includes(b.branch.nodes[0])) {
						b.branch.energized = b.branch.energized > 0 ? 1 : 2;
						queue.push(b.branch.nodes[0]);
						b.branch.nodes.reverse();
					}
				}
			}
			oldNodes.add(node);
		}
	}
	emptyState() {
		for(let node of this.nodes) {
			node.status = 0;
		}
		this.branches.forEach(branch => branch.energized = 0);
	}
	setState(hist) {
		this.emptyState();

		let lastState = null;
		let lastEnergizedBranches;
		for(let s of hist) {
			this._setState(s);
			if (lastState == null) {
				lastState = s;
				continue;
			}
			let diffs = [
				// { i: index, potentials: [] }
			];
			for (let i = 0; i < s.length; ++i) {
				if (lastState[i] != s[i] && s[i] != "D") {
					let potentials = [];
					for(let b of this.nodes[i].branches) {
						let other = b.branch.nodes[0] == i ?
								b.branch.nodes[1] : b.branch.nodes[0];
						if (lastState[other] == s[i]) {
							potentials.push(other);
						}
					}
					if (potentials.length > 0) {
						diffs.push({
							i: i,
							potentials: potentials,
						});
					}
					// TODO: handle cases where a bus gets energized from externalBranch
					// but a neighboring bus gets energized from somewhere else
					// Probably need to get sourceNames from server.
					// Currently there's no mapping from sourceNames -> source IDs in the
					// client side.
					// TODO: Another limitation: When TG powers DER-powered buses,
					// directions may change completely.
				}
			}

			lastEnergizedBranches = [];
			while (diffs.length > 0) {
				let mindiff = 0;
				for (let i = 1; i < diffs.length; ++i) {
					if (diffs[i].potentials.length < diffs[mindiff].potentials.length) {
						mindiff = i;
					}
				}
				mindiff = diffs.splice(mindiff, 1)[0];
				let source = mindiff.potentials[0];
				let target = mindiff.i;
				for(let b of this.nodes[target].branches) {
					if (b.branch.nodes[0] == target && b.branch.nodes[1] == source) {
						b.branch.nodes.reverse();
						b.branch.energized = 1;
						lastEnergizedBranches.push(b.branch);
						break;
					} else if (b.branch.nodes[0] == source && b.branch.nodes[1] == target) {
						b.branch.energized = 1;
						lastEnergizedBranches.push(b.branch);
						break;
					}
				}
			}
			lastState = s;
		}
		for (let b of lastEnergizedBranches) {
			b.energized = 2;
		}

		if(Settings.colorized && Settings.colorizationTarget === "cpf") {
			this.calculateCumulativePfs();
		}
		this.rerender();
		if(BottomRightPanel.contentInfo && BottomRightPanel.contentInfo.node) {
			this.showNodeInfo(BottomRightPanel.contentInfo.node);
		}
	}
	calculateCumulativePfs() {
		if(policyView) {
			let cpfs = policyView.policy.cumulativePfs();
			for(let i = 0; i<cpfs.length; ++i) {
				this.nodes[i].cpf = cpfs[i];
			}
		} else {
			console.log("Warning: no policyView in calculateCumulativePfs");
			// TODO: to fully fix this, generate and send MDP before policy
			for(let i = 0; i<this.nodes.length; ++i) {
				this.nodes[i].cpf = this.nodes[i].pf;
			}
		}
	}
	cancelUpdate() {
		BottomRightPanel.hide();
		for(let node of this.nodes) {
			if("originalPf" in node) node.pf = node.originalPf;
		}
	}
	onPanelHide() {
		this.lastEdit.active = false;
	}
} //end Graph


window.addEventListener("keydown", function(event) {
	switch(event.key){
		case "F1":
			graph.setMode(0);
			break;
		case "F2":
			graph.setMode(1);
			break;
		default:
			graph.handleKeyDown(event);
			break;
	}
});


