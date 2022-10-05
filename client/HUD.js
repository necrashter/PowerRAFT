const BottomRightPanel = document.getElementById("BottomRightPanel");
const BottomRightPanelContent = BottomRightPanel.querySelector(".content");

const Styles = [
	{ name: "CyberPunk", folder: "cyberpunk-style" },
	{ name: "Lab Dark", folder: "lab-dark-style" },
	{ name: "Lab", folder: "lab-style" },
	{ name: "Medieval", folder: "medieval-style" },
];

function changeStyle(folderName) {
	document.getElementById("index-css").href = folderName+"/index.css";
	document.getElementById("graph-css").href = folderName+"/graph.css";
	document.getElementById("custom-select-css").href= folderName+"/CustomSelect.css";
	document.getElementById("custom-checkbox-css").href = folderName+"/CustomCheckbox.css";
	// document.getElementById("spinner-css").href = folderName+"/Spinner.css";
}


BottomRightPanel.show = function(info=null) {
	BottomRightPanel.contentInfo = info;
	BottomRightPanel.classList.remove("hidden");
}

BottomRightPanel.hide = function() {
	BottomRightPanel.contentInfo = null;
	BottomRightPanel.classList.add("hidden");
	if(graph) graph.onPanelHide();
}

function createTextInput(target, name, value) {
	let div = target.append("div");
	div.append("label").attr("for", name).text(name+":");
	let input = div.append("input").attr("type", "text").property("value", value);
	return input;
}

function createSelectBox(target, data, name, value) {
	let info = {value: value};
	let currentName = data.find(d => d.value == value).name;
	let div = target.append("div").style("display", 'flex');
	div.append("label").text(name+":");
	var innerDiv;
	const wrapperDiv = div.append("div")
		.attr("class","CustomSelect")
		.style("flex-grow", 1);
	var headDiv = wrapperDiv.append("div")
		.attr("class", "CustomSelectHead")
		.text(currentName)
		.on("click", function() {
			innerDiv.classList.toggle("open");
		}).node();
	const ul = wrapperDiv.append("div").attr("class", "CustomSelectList")
		.append("div");
	innerDiv = ul.node();

	ul.selectAll("div").data(data).join("div")
		.attr("class", "CustomSelectElement")
		.text(d => d.name)
		.on("click", d => {
			info.value = d.value;
			headDiv.innerText=d.name;
			innerDiv.classList.remove("open");
		});
	return info;
}

function createCustomSelectBox(div, data, currentIndex=0, zIndex=1) {
	let current = data[currentIndex];
	var innerDiv;
	const wrapperDiv = div.append("div")
		.attr("class","CustomSelect")
		.style("flex-grow", 1)
		.style("z-index", zIndex);
	var headDiv = wrapperDiv.append("div")
		.attr("class", "CustomSelectHead")
		.html(current.name)
		.on("click", function() {
			innerDiv.classList.toggle("open");
		}).node();
	const ul = wrapperDiv.append("div").attr("class", "CustomSelectList")
		.append("div");
	innerDiv = ul.node();

	ul.selectAll("div").data(data).join("div")
		.attr("class", "CustomSelectElement")
		.html(d => d.name)
		.on("click", d => {
			d.func();
			headDiv.innerHTML=d.name;
			innerDiv.classList.remove("open");
		});
}

function createCheckbox(target, label, onChange) {
	let id = label.toLowerCase().replace(/\s+/g , "-");
	let div = target.append("div").classed("customCheckbox", true);
	let checkbox = div.append("input")
		.attr("id", id)
		.attr("type", "checkbox")
		.on("change", () => {
			onChange(checkbox.node().checked);
		});
	//checkbox.node().checked = Settings.animateAnts;
	div.append("label")
		.attr("for", id)
		.text(label);
	return div;
}

function addSpinnerDiv(div) {
	let out = div.append("div").classed("spinnerWrapper", true);
	out.append("div").classed("spinner", true);
	return out;
}



function AdvancedCopy(text){
	navigator.clipboard.writeText(text);
}

function getGraphDiagram() {
	function euclid(a, b) {
		return Math.sqrt(Math.pow(a[0]-b[0], 2) + Math.pow(a[1]-b[1], 2));
	}
	let output = `
\\begin{tikzpicture}[auto,node distance=8mm,>=latex,font=\\small]
\\tikzstyle{round}=[thick,draw=black,circle]
\\begin{scope}[local bounding box=graph]
	`;
	let distanceMul = 2;
	let center = graph.nodes[0].latlng;
	let totalDistance = 0;
	for(let edge of graph.branches) {
		totalDistance += euclid(graph.nodes[edge.nodes[0]].latlng, graph.nodes[edge.nodes[1]].latlng);
	}
	totalDistance /= graph.branches.length;
	distanceMul /= totalDistance;
	for(let node of graph.nodes) {
		let pos = [...node.latlng];
		pos[0] -= center[0];
		pos[1] -= center[1];
		pos[0] *= distanceMul;
		pos[1] *= distanceMul;
		let i = node.index;
		let external = "";
		if(node.externalBranches) {
			let sources = node.externalBranches.map(b => "E_{"+b.branch.source+"}");
			if(sources.length > 0)
				external = `, label=0:{Connected to $${sources.toString()}$}`;
		}
		pos.reverse();
		output += `
\\node[round, label=south:$${node.pf.toFixed(3)}$${external}] (${i}) at (${pos.toString()}) {${i+1}};\
`;
	}
	for(let edge of graph.branches) {
		output+=`
\\draw[-] (${edge.nodes[0]}) -- (${edge.nodes[1]});\
`;
	}
	output += `
\\end{scope}
\\end{tikzpicture}
	`;
	AdvancedCopy(output);
	return output;
}

var krinkOut= "";
function krink() {
	krinkOut = policyView.policy.createGraph2();
	/*
	let output = "";
	while(policyView.policy.nextStateAvailable()) {
		output += policyView.policy.createGraph();
		policyView.policy.nextState();
		policyView.policyNavigator(); // refresh
	}
	AdvancedCopy(output);
	krinkOut=output;
	return output;
	*/
}


function copyBenchmarkTableMd() {
	let benchmark = policyView.policy.benchmark;
	if (!benchmark) {
		console.error("No benchmark data available!");
		return;
	}
	header = {
		name: "Optimizations",
		elapsed: "Elapsed Time",
		states: "States",
		value: "Value",
	};
	cols = Object.keys(header);
	lens = {};
	for (let key in header) {
		lens[key] = header[key].length;
	}
	for (let b of benchmark) {
		for (let key in lens) {
			lens[key] = Math.max(lens[key], b[key].toString().length);
		}
	}
	console.log(lens);
	let lines = [];
	lines.push(`| ${header.name.padEnd(lens.name)} | ${header.elapsed.padEnd(lens.elapsed)} | ${header.states.padEnd(lens.states)} | ${header.value.padEnd(lens.value)} |`);
	lines.push(`|${'-'.repeat(2+lens.name)}|${'-'.repeat(2+lens.elapsed)}|${'-'.repeat(2+lens.states)}|${'-'.repeat(2+lens.value)}|`);
	for (let b of benchmark) {
		lines.push(`| ${b.name.toString().padEnd(lens.name)} | ${b.elapsed.toString().padEnd(lens.elapsed)} | ${b.states.toString().padEnd(lens.states)} | ${b.value.toString().padEnd(lens.value)} |`);
	}
	let s = lines.join('\n');
	AdvancedCopy(s);
}
