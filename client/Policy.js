
var policyView = null;
const defaultHorizon = 30;

const ACTION_OFFSET = 0;//-1;

const TeamColors = d3.schemeCategory10;
const TeamColorNames = [
	"Blue", "Orange", "Green",
	"Red", "Purple", "Brown",
	"Pink", "Gray", "Yellow",
	"Turquoise"
];
const TeamIcons = d3.schemeCategory10.map(color => {
	const svgTemplate = `
	<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" class="marker">
	  <path fill-opacity=".25" d="M16 32s1.427-9.585 3.761-12.025c4.595-4.805 8.685-.99 8.685-.99s4.044 3.964-.526 8.743C25.514 30.245 16 32 16 32z"/>
	  <path fill="${color}" stroke="#000" d="M15.938 32S6 17.938 6 11.938C6 .125 15.938 0 15.938 0S26 .125 26 11.875C26 18.062 15.938 32 15.938 32zM16 6a4 4 0 100 8 4 4 0 000-8z"/>
	</svg>`;

	return L.divIcon({
		className: "marker",
		html: svgTemplate,
		iconSize: [40, 40],
		iconAnchor: [20, 40]
	});
});

function getNumberIcon(name) {
	let width = name.toString().length*7+20;
	return L.divIcon({
		className: 'divIcon',
		html: `<div class='numberMarker'>${name}</div>`,
		iconSize: [width,50],
		iconAnchor: [width/2,0]
	});
}

function getDescriptionIcon(str, neg=false) {
	let width = str.toString().length*6+40;
	let classname = "descMarker";
	if(neg) classname += " neg";
	return L.divIcon({
		className: 'divIcon',
		html: `<div class='${classname}'>${str}</div>`,
		iconSize: [width,50],
		iconAnchor: [width/2,0]
	});
}

function getNodeDescriptionIcon(str, classname, customStyle="") {
	return L.divIcon({
		className: 'divIcon',
		html: `<div class='${classname}' style='${customStyle}'>${str}</div>`,
		iconSize: [200, 200],
		iconAnchor: [100, 100]
	});
}

/**
 * Get team struct for creating a team under the mouse cursor.
 * Creates on bus if possible, otherwise creates based on latlng.
 */
function getMouseTeam(event) {
	let team = { latlng: [event.latlng.lat, event.latlng.lng] };
	let lastHover = graph.lastHover;
	if (lastHover.hovered && lastHover.type === "node") {
		team.index = lastHover.data.index;
	}
	return team;
}


class InteractivePolicy {
	/**
	 * policy is the direct output from server
	 */
	constructor(_graph, policy, initialize=true) {
		this.graph = graph;
		for(let key in policy) {
			this[key] = policy[key];
		}
		if(initialize) {
			this.end = false; // if true (No more actions available)
			this.previousStates = [];
			this.setState(0);
		}
	}
	setAction(actionNum, next) {
		this.actionNum = actionNum;
		this.action = this.actions[actionNum];
		if(this.action) {
			this.end = this.action.length == 1 && this.action[0][0]+ACTION_OFFSET == this.state;
		} else {
			this.end = true;
		}
		// set graph to the next state
		if(next === null || this.end) {
			let hist = this.previousStates.map(s => this.states[s.state]);
			hist.push(this.states[this.state]);
			if(this.end) hist.push(this.states[this.state]);
			this.graph.setState(hist);
		}
		else this.setNext(next);
	}
	setState(state, next = 0) {
		this.state = state;
		this.actions = this.transitions[this.state];
		let nextAction = this.policy[this.state];
		this.setAction(nextAction, next);
	}
	setStateAndAction(state, action, next = 0) {
		this.state = state;
		this.actions = this.transitions[this.state];
		this.setAction(action, next);
	}
	setNext(i) {
		// represents which transition to take in the current action
		this.next = i;
		let nextState = this.states[this.action[this.next][0] + ACTION_OFFSET];
		let hist = this.previousStates.map(s => this.states[s.state]);
		hist.push(this.states[this.state]);
		hist.push(nextState);
		this.graph.setState(hist);
	}
	getCurrentState() {
		return this.states[this.state];
	}
	getNextState() {
		return this.states[this.action[this.next][0] + ACTION_OFFSET];
	}
	getCurrentTeams() {
		return this.teams[this.state];
	}
	getNextTeams() {
		return this.teams[this.action[this.next][0] + ACTION_OFFSET];
	}
	describeAction(actionNum = null) {
		if(actionNum == null) actionNum =this.actionNum;
		let header =  "Action #"+actionNum;
		let energizationInfo = "";
		let teamInfo = "";
		let valueInfo = "";
		if(actionNum == this.policy[this.state]) {
			header += " (Policy)";
		}
		if(this.values) {
			let value = this.values[this.state][actionNum];
			valueInfo = `<span style="float:right;">${value.toFixed(3)}</span>`;
		}
		let action = this.actions[actionNum];
		let currentState = this.states[this.state];
		let nextState = this.states[action[0][0] + ACTION_OFFSET];
		let energized = currentState.map((b, i) => b != nextState[i] ? i : null).filter(a => a != null);
		if(energized.length > 0) {
			energizationInfo = "&emsp;Node(s): " + energized.join(", ") + "<br/>";
		}
		if(this.teams) {
			let currentTeam = this.teams[this.state];
			let nextTeam = this.teams[action[0][0] + ACTION_OFFSET];
			let teamInfos = [];
			for(let i = 0; i < currentTeam.length; ++i) {
				if(nextTeam[i].target != null) {
					teamInfos.push((i+1)+" to "+nextTeam[i].target);
				} else if(currentTeam[i].node != nextTeam[i].node) {
					teamInfos.push((i+1)+" to "+nextTeam[i].node);
				}
			}
			if(teamInfos.length > 0) {
				teamInfo = "&emsp;Team "+teamInfos.join(", ")+"<br/>";
			}
		}
		return `
		<b>${header}</b><br/>
		${valueInfo}
		${energizationInfo}
		${teamInfo}
		`;
	}
	describeTransition(action, i) {
		let nextState = this.states[action[0] + ACTION_OFFSET];
		let currentState = this.states[this.state];
		let list = "";
		for(let i = 0; i < currentState.length; ++i) {
			if(nextState[i] != currentState[i]) {
				list += `<div>Node #${i}: ${nextState[i]}</div>`;
			}
		}
		if (list == "") {
			list = "Deterministic Transition";
		}
		//return `<div><b>Transition #${i}</b> <br/>`+this.states[action[0] - 1].toString()+"</div>";
		return `
			<div><b>Transition #${i}</b> <div>${list}</div> </div>
			<div class="rightFlexFloat">P = ${action[1].toFixed(3)}</div>
			`;
	}
	nextState() {
		this.previousStates.push({
			state: this.state,
			actionNum: this.actionNum,
			next: this.next,
		});
		this.setState(this.action[this.next][0] + ACTION_OFFSET);
	}
	previousState() {
		let prev = this.previousStates.pop();
		if(prev === undefined) throw new Error("There's no previous state");
		this.setStateAndAction(prev.state, prev.actionNum, prev.next);
	}
	nextStateAvailable() {
		return !this.end;
	}

	/**
	 * Calculates the energization probability of all nodes
	 * recursive solution
	 */
	_allEnergizationProb(state) {
		let vec = this.states[state].map(s => {
			if(s === "D") return 0;
			if(s !== "U") return 1;
			return null;
		});
		let actions = this.transitions[state];
		let action = actions[this.policy[state]];
		if(!action) return vec.map(s => s==null ? 0 : s);
		if(action.length == 1 && action[0][0]-1 == state)
			return vec.map(s => s==null ? 0 : s);
		return action.map(a => {
			if(a[0]-1 == state) vec.map(() => 0);
			let mul = vec.map(s => s==null ? a[1] : a[1]*s);
			return this._allEnergizationProb(a[0]-1).map((p,i) => p * mul[i]);
		}).reduce((a,b) => a.map((n,i) => n+b[i]), vec.map(() => 0));
	}
	allEnergizationProb() {
		return this._allEnergizationProb(this.state);
	}
	cumulativePfs() {
		return this.allEnergizationProb().map(a => 1-a);
	}
	/**
	 * Calculates the energization probability of the node at nodeIndex
	 * in "depth" number of steps
	 * recursive solution
	 */
	_energizationProbability(state, nodeIndex, depth) {
		let s = this.states[state][nodeIndex]; //current state of the node
		console.log(state)
		if(depth==0) {
			return (s === "D" || s === "U") ? 0 : 1;
		}
		if(s === "D") return 0;
		if(s !== "U") return 1;
		let actions = this.transitions[state];
		let action = actions[this.policy[state]];
		if(!action) return 0;
		if(action.length == 1 && action[0][0]-1 == state) return 0;
		return action.map(a => {
			if(a[0]-1 == state) return 0;
			return a[1]*this._energizationProbability(a[0]-1, nodeIndex, depth-1);
		}).reduce((a,b) => a+b, 0);
	}
	/**
	 * Calculates the energization probability of the node at nodeIndex
	 * iterative solution to calculate all steps
	 */
	_energizationProbabilities(state0, nodeIndex) {
		let queue = [{
			state: state0,
			p: 1,
			depth: 0,
		}];
		let results = [0];
		while(queue.length > 0) {
			let {state, p, depth} = queue.shift(); //fifo
			if(typeof results[depth] === "undefined")
				results[depth] = results[depth-1];
			let s = this.states[state][nodeIndex]; //current state of the node
			if(s === "D") continue; // nothing to see
			if(s !== "U") results[depth] += p; //energized
			else {
				// energized
				let actions = this.transitions[state];
				let action = actions[this.policy[state]];
				if(!action) continue;
				for(let a of action) {
					if(a[0]-1 == state) continue;
					queue.push({
						state: a[0]-1,
						p: p*a[1],
						depth: depth+1,
					});
				}
			} //end else
		} // end while
		return results;
	}
	isEnergized(nodeIndex) {
		let status = this.states[this.state][nodeIndex];
		console.log(status);
		return status != "D" && status != "U";
	}
	energizationProbabilities(nodeIndex) {
		return this._energizationProbabilities(this.state, nodeIndex);
	}
	energizationProbability(nodeIndex, depth) {
		return this._energizationProbability(this.state, nodeIndex, depth);
	}
	update(policy, next = 0) {
		Object.assign(this, policy);
		this.setAction(this.policy[this.state], next);
	}

	/**
	 * for paper
	 */
	createGraph2() {
		const preci = 6;
		let s = this.state+1; // 1 indexing used in dists and values
		let actions = this.actions;
		let dists = this.dists[s]; // 1 indexing
		let probs = this.probs[s]; // 1 indexing
		let values = this.values[s];
		// NOTE: assumes that no two actions can end in the same state
		let endStateLength = Object.values(actions).map(action => action.length)
			.reduce((i, c) => i+c, 0);
		let currentState = this.states[s-1];
		let state = "{(" + this.states[s-1].toString().replaceAll(/TG\d+/g, "E") + ")}";
		let output = "";
		let lll = "l|";
		let optHeader = "Action ";
		let optBody = "";
		for(let actKey in actions) {
			let action = actions[actKey];
			let actionName = "\\{" + this.states[action[0][0]-1].map((s, i) => s == currentState[i] ? -1 : i+1).filter(s => s != -1).join(", ")+"\\}";
			let ah = `\\multirow{${action.length}}{*}{$${actionName}$}`;
			output += ` \\hline
			`;
			for(let endKey in action) {
				let end = action[endKey];
				let e = end[0];
				let st = "{(" + this.states[e-1].toString().replaceAll(/TG\d+/g, "E") + ")}";
				let p = end[1].toFixed(3);
				output += `${ah} & ${p} & $s_{${e}} = ${st}$ \\\\
				`;
				ah = "";
			}
			let firstEnd = action[0][0];
			let lastEnd = action[action.length-1][0];
			let value = values[actKey].toFixed(preci);
			let cols = [];
			for(let goal in probs) {
				let prob = probs[goal][actKey].toFixed(preci);
				cols.push(`$${prob}$`);
				if(actKey == 1) {
					optHeader += ` & $P_{\\infty, ${goal}}^{\\star}$ `
					lll += "l ";
				}
			}
			for(let goal in dists) {
				let dist = dists[goal][actKey].toFixed(preci);
				cols.push(`$${dist}$`);
				if(actKey == 1) {
					optHeader += ` & $C_{\\infty, ${goal}}^{\\star}$ `
					lll += "l ";
				}
			}
			if(actKey == 1) {
				optHeader += ` & $V$ `
				lll += "l ";
			}
			cols.push(`$${value}$`);
			optBody += `$${actionName}$ & ${cols.join(" & ")} \\\\\n`;
		}
		let output3 = `
		\\begin{table}[h]
	\\centering
	\\caption{Transitions from $s_{${s}} = ${state}$}
	\\begin{tabular} {c|c|c}
		Action & Probability & Next State \\\\
		${output}
	\\end{tabular}
	\\label{transitions:s${s}}
\\end{table}

\\begin{table}[h]
	\\centering
	\\caption{Optimal values for $s_{${s}}$}
	\\begin{tabular}{ ${lll}}
		${optHeader} \\\\
		\\hline
		${optBody}
\\end{tabular}
\\label{distances:s${s}}
\\end{table}
		`;
		AdvancedCopy(output3);
		return output3;
	}
	/**
	 * for paper
	 */
	createGraph() {
		let width = 40;
		let widthMod = -0.01;
		let s = this.state+1; // 1 indexing used in dists and values
		let actions = this.actions;
		let dists = this.dists[s]; // 1 indexing
		let values = this.values[s];
		// NOTE: assumes that no two actions can end in the same state
		let endStateLength = Object.values(actions).map(action => action.length)
			.reduce((i, c) => i+c, 0);
		let startHeight = totalHeight/2;
		let currentHeight = startHeight;
		let matrix = ``;

		let state = "{\\small(" + this.states[s-1].toString().replaceAll("TG", "E_") + ")}";
		let output = `
\\begin{tikzpicture}[auto,node distance=8mm,>=latex,font=\\small]
\\tikzstyle{round}=[thick,draw=black,circle]

\\begin{scope}
\\node[round, label=west:$${state}$] (s${s}) {$s_{${s}}$};
		`;
		for(let actKey in actions) {
			output += `%% ACTION ${actKey}\n`;
			let action = actions[actKey];
			for(let endKey in action) {
				let end = action[endKey];
				let e = end[0];
				let w = width + (widthMod* Math.pow(currentHeight, 2));
				let st = "{\\small (" + this.states[e-1].toString().replaceAll("TG", "E_") + ")}";
				let p = end[1].toFixed(3);
				output += `
\\node[round,above right=${currentHeight}mm and ${w}mm of s${s}, label=0:$${st}$] (s${e}) {$s_{${e}}$};
\\draw[->] (s${s}) -- (s${e});
\\draw (s${s}) -- (s${e}) node [pos=.7, above, sloped] (TextNode) {\\scriptsize $p \\approx ${p}$};
				`;
				currentHeight -= height;
			}
			let firstEnd = action[0][0];
			let lastEnd = action[action.length-1][0];
			let value = values[actKey].toFixed(3);
			output += `
\\path pic[draw, angle radius=10mm,"$a_{${actKey}}$",angle eccentricity=1.5]
			{angle = s${lastEnd}--s${s}--s${firstEnd}};
			`;
			let cols = [];
			for(let goal in dists) {
				let dist = dists[goal][actKey].toFixed(3);
				cols.push(`$D^{[${goal}]}(s_{${s}}, a_{${actKey}}) = ${dist}$`);
			}
			cols.push(`$V(s_{${s}}, a_{${actKey}}) = ${value}$`);
			matrix += `${cols.join(", & ")} \\\\\n`;
		}
		output += `
\\end{scope}
\\end{tikzpicture}

\\begin{center}
\\begin{tabular}{ ${Object.keys(dists).map(() => "l").join(" ")} l }
${matrix}
\\end{tabular}
\\end{center}
		`;
		AdvancedCopy(output);
		return output;
	}
}

function loadPolicy(div, graph, policy, options={}) {
	options.prelude = d => {
		let ul = d.append("ul");
		if(options.priorities) {
			ul.append("li").text("Prioritized nodes: ")
			.append("ul").selectAll("li").data(options.priorities).join("li")
				.text((p,i) => 
					"Class "+(i+1)+"("+(p.maxmin ? "Maxmin" : "Minmin")+"): "+
					p.nodes.map(i => getNodeName(graph.nodes[i])).join(", ")
				);
		} else {
			ul.append("li").text("No prioritization");
		}
		let horizon = "Default";
		if(options.conf) horizon = options.conf.horizon;
		ul.append("li").text("Horizon: "+horizon);
	}
	div.html("");
	div.append("h3").text("Select Mode");
	div.append("p").text(`Interactive Mode allows you to select the
				result of each step as you run the policy.`);
	div.append("div").classed("blockButton", true)
		.text("Interactive Mode")
		.on("click", () => {
			policyView = new InteractivePolicyView(graph, div,
				new InteractivePolicy(graph, policy), options);
		});
	div.append("p").text(`In linear mode you will see the best case,
				where the activation of every bus is successful.`);
	div.append("div").classed("blockButton", true)
		.text("Linear Mode")
		.on("click", () => {
			policyView = new LinearPolicyView(graph, div,
				new LinearPolicy(graph, policy), options);
		});
}


function requestSaveExp(graph, settings) {
	let request = {
		graph: graph.serialize(),
	};
	Object.assign(request, settings);
	return Network.post("/save-experiment", JSON.stringify(request));
}

/**
 * Save the policy for an experiment.
 */
function saveAsExperiment(div, graph, settings) {
	console.log("Requesting to save experiment...");
	div.html("");
	addSpinnerDiv(div).append("p").text("Waiting response from server...");
	requestSaveExp(graph, settings).then(response => {
		response = JSON.parse(response);
		console.log("Response:", response);
		div.resetDiv(settings.name);
		if (response.successful) {
			div.append("b").text("Successfully saved experiment as:");
			div.append("p").text(settings.name);
		} else {
			div.append("b").text("Failed to save experiment!")
				.style("color","red");
			div.append("p").text(response.error)
				.style("color","red");
		}
	}).catch(error => {
		div.resetDiv(settings.name);
		div.append("b").text("Failed to save experiment!")
			.style("color","red");
		div.append("p").html(error)
			.style("color","red");
	});
}

function requestPolicy(graph, settings) {
	let request = {
		graph: graph.serialize(),
	};
	Object.assign(request, settings);
	return Network.post("/policy", JSON.stringify(request));
}

var lastRequestedPolicy = null;

function requestNewPolicy(div, graph, settings={}) {
	lastRequestedPolicy = [div, graph, settings];
	console.log("requesting new policy...");
	div.html("");
	addSpinnerDiv(div).append("p").text("Waiting response from server...");
	requestPolicy(graph, settings).then(response => {
		let policy = JSON.parse(response);
		loadPolicy(div, graph, policy, settings);
	}).catch(error => {
		div.html("");
		div.append("b").text("Failed to get policy")
			.style("color","red");
		div.append("p").text(error)
			.style("color","red");
		div.append("div").classed("blockButton", true)
			.text("Retry")
			.on("click", () => requestNewPolicy(...lastRequestedPolicy));
		div.append("div").classed("blockButton", true)
			.text("Go Back")
			.on("click", () => selectPolicyView(div, graph));
	});
}
/**
 * Gets the premade policy from server if available,
 * otherwise requests new policy
 */
function getPolicy(div, graph) {
	div.html("");
	addSpinnerDiv(div).append("p").text("Please wait...");
	if(graph.solutionFile && !graph.dirty) {
		// force get new version
		let url = graph.solutionFile + "?time="+ Date.now();
		Network.get(url).then(response => {
			console.log("Fetched premade policy:",graph.solutionFile);
			loadPolicy(div, graph, JSON.parse(response));
		}).catch(error => {
			console.error("Error while getting premade policy",
				graph.solutionFile,":",error);
			requestNewPolicy(div, graph);
		});
	} else {
		requestNewPolicy(div, graph);
	}
}


function policySettings(div, graph){
	div.html("");
	div.style("opacity", 0).transition().duration(500).style("opacity", 1);
	let errorDiv = div.append("p").text("")
		.style("color", "red");
	let tabNames = ["General", "Teams", "Prioritization"];
	let tabButtonsDiv = div.append("div").classed("tabButtons", true);
	let tabs = [
		div.append("div"),
		div.append("div").style("display", "none"),
		div.append("div").style("display", "none"),
	];
	let selectedTab;
	function selectTab(index) {
		if(selectedTab == index) return;
		selectedTab = index;
		tabButtons.each(function(d, i) {
			if (i == index) {
				tabs[i].style("display", "");
				d3.select(this).classed("disabled", true);
			} else {
				tabs[i].style("display", "none");
				d3.select(this).classed("disabled", false);
			}
		});
	}
	let tabButtons = tabButtonsDiv.selectAll("div")
		.data(tabNames).join("div").classed("blockButton", true)
		.text(t => t)
		.on("click", (text, index) => {
			selectTab(index);
		});
	selectTab(0);

	// GENERAL
	let horizon = createTextInput(tabs[0], "Horizon", defaultHorizon);
	let benchmark_enabled = false;
	createCheckbox(tabs[0], "Benchmark", val => {
		benchmark_enabled = val;
	});

	// TEAMS
	let teams = [];
	let teamTypeNames = ["Instant", "Continuous", "Discrete"];
	let selectedTeamType = 2;
	let selectedTeam = -1;
	let teamButtons = tabs[1].append("div").classed("policyControls", true);
	let teamDiv = tabs[1].append("div");
	teamButtons.append("div").classed("blockButton", true)
		.text("Add")
		.on("click", () => {
			if(teams.length == 10) {
				return;
			}
			let pos = Map.getCenter();
			teams.push({
				latlng: [
					pos.lat - 0.025 + 0.05 * Math.random(),
					pos.lng - 0.025 + 0.05 * Math.random(),
				]
			});
			selectTeam(null, teams.length - 1);
		});
	teamButtons.append("div").classed("blockButton", true)
		.text("Delete")
		.on("click", () => {
			teams.splice(selectedTeam, 1);
			teamDiv.selectAll(".list-elem.selected")
				.style("max-height", 0)
				.style("padding", 0)
				.transition().duration(500)
				.style('opacity', 0)
				.style('transform', "translateX(80%)")
				.on('end', function() {
					d3.select(this).remove();
					selectTeam(null, Math.min(selectedTeam, teams.length - 1));
				});
		});
	let teamTypeSelector;
	{
		teamTypeSelector = teamButtons.append("div").attr("class","CustomSelect")
			.style("flex-grow", 1);
		let innerDiv;
		let teamTypeHead = teamTypeSelector.append("div")
			.attr("class", "CustomSelectHead")
			.text(teamTypeNames[selectedTeamType])
			.on("click", function(_) {
				innerDiv.classList.toggle("open");
			});
		const ul = teamTypeSelector.append("div").attr("class", "CustomSelectList").append("div");
		innerDiv = ul.node();
		ul.selectAll("div")
			.data(teamTypeNames).join("div")
			.attr("class", "CustomSelectElement")
			.text(name => name)
			.on("click", (name, i) => {
				teamTypeHead.text(name);
				selectedTeamType = i;
				innerDiv.classList.remove("open");
			});
	}
	function selectTeam(_, i) {
		selectedTeam = i;
		renderTeams();
		createTeamMarkers();
	}
	function renderTeamInfo(d, i) {
		let item = d3.select(this);
		item.classed("selected", i == selectedTeam);
		item.select(".item-header").text("Team #" + (i+1));
		if ('index' in d) {
			item.select(".item-info").text("On bus "+getNodeName(graph.nodes[d.index]));
		} else if ('latlng' in d) {
			item.select(".item-info").text(d.latlng.map(a => a.toFixed(4)).join(", "));
		} else {
			item.select(".item-info").text("TEAM STATE CORRUPT!");
		}
		item.select(".rightFlexFloat").text(TeamColorNames[i]);
	}
	function renderTeams() {
		if(teams.length > 0) {
			teamDiv.classed("selectList", true).classed("wide", true);
			teamDiv.selectAll("h4").remove();
			teamTypeSelector.style("display", "");
			teamDiv.selectAll(".list-elem")
				.data(teams)
				.join(
					function(enter) {
						let item = enter.append("div")
							.classed("list-elem", true)
							.style("opacity", 0)
							.style("max-height", "10em")
							.style("transition", "max-height 0.5s, padding 0.5s")
						let left = item.append("div");
						left.append("b").classed("item-header", true);
						left.append("div").classed("item-info", true);
						item.append("div").classed("rightFlexFloat", true);
						return item;
					},
					function(update) {
						return update;
					},
					function(exit) {
						return exit
							.transition().duration(500)
							.style('opacity', 0)
							.style('transform', "translateX(80%)")
							.on('end', function() {
								d3.select(this).remove();
							});
					}
				)
				.each(renderTeamInfo)
				.on("click", selectTeam)
				.transition().duration(500)
				.style("opacity", 1)
		} else {
			teamDiv.classed("selectList", false).classed("wide", false);
			teamDiv.html("").append("h4").text("No Teams");
			teamTypeSelector.style("display", "None");
		}
	}
	let teamMarkers = null;
	let teamMarkerLayer = null;
	let createTeamMarkers = () => {
		if(teamMarkerLayer) teamMarkerLayer.remove();
		if(selectedTeam < 0) return;
		teamMarkers = teams.map((team, i) => {
			let color = TeamColors[i];
			let m = L.marker(team.latlng, {
				icon: TeamIcons[i],
				color: color,
				pane: "teams",
			});
			m.on("click", () => selectTeam(team, i));
			return m;
		});
		teamMarkerLayer = L.featureGroup(teamMarkers);
		teamMarkerLayer.addTo(Map);
	};
	selectTeam(null, -1);

	// PRIORITY
	let priorityClasses = [];
	// ids of the prioritized node
	let priorities = {};
	let selectedPriorityClass = -1;
	let priorityButtons = tabs[2].append("div").classed("policyControls", true);
	let priorityClassDiv = tabs[2].append("div");
	let nodeListWrapper = tabs[2].append("div");
	let nodeListButtons = nodeListWrapper.append("div").classed("policyControls", true);
	let nodeList = nodeListWrapper.append("div").classed("selectList", true);
	priorityButtons.append("div").classed("blockButton", true)
		.text("Add")
		.on("click", () => {
			priorityClasses.push({
				nodes: new Set(),
				maxmin: true,
			});
			selectPriorityClass(null, priorityClasses.length - 1);
		});
	priorityButtons.append("div").classed("blockButton", true)
		.text("Delete")
		.on("click", () => {
			priorityClasses.splice(selectedPriorityClass, 1);
			priorityClassDiv.selectAll(".list-elem.selected")
				.style("max-height", 0)
				.style("padding", 0)
				.transition().duration(500)
				.style('opacity', 0)
				.style('transform', "translateX(80%)")
				.on('end', function() {
					d3.select(this).remove();
					selectPriorityClass(null, Math.min(selectedPriorityClass, priorityClasses.length - 1));
				});
		});
	priorityButtons.append("div").classed("blockButton", true)
		.text("Change Type")
		.on("click", () => {
			if(selectedPriorityClass < 0) return;
			priorityClasses[selectedPriorityClass].maxmin = !priorityClasses[selectedPriorityClass].maxmin;
			renderPriorityClasses();
		});
	function selectPriorityClass(_, i) {
		selectedPriorityClass = i;
		renderPriorityClasses();
		createNodeMarkers();
	}
	function renderPriorityClass(d, i) {
		let item = d3.select(this);
		item.classed("selected", i == selectedPriorityClass);
		item.select(".item-header").text("Priority Class #" + (i+1));
		item.select(".item-info").text("Nodes: " + (d.nodes.size > 0 ? [...d.nodes].join(", ") : "None"));
		item.select(".rightFlexFloat").text(d.maxmin ? "MinMax" : "MinMin");
	}
	function renderPriorityClasses() {
		if(priorityClasses.length > 0) {
			priorityClassDiv.classed("selectList", true).classed("wide", true);
			priorityClassDiv.selectAll("h4").remove();
			priorityClassDiv.selectAll(".list-elem")
				.data(priorityClasses)
				.join(
					function(enter) {
						let item = enter.append("div")
							.classed("list-elem", true)
							.style("opacity", 0)
							.style("max-height", "10em")
							.style("transition", "max-height 0.5s, padding 0.5s")
						let left = item.append("div");
						left.append("b").classed("item-header", true);
						left.append("div").classed("item-info", true);
						item.append("div").classed("rightFlexFloat", true);
						return item;
					},
					function(update) {
						return update;
					},
					function(exit) {
						return exit
							.transition().duration(500)
							.style('opacity', 0)
							.style('transform', "translateX(80%)")
							.on('end', function() {
								d3.select(this).remove();
							});
					}
				)
				.each(renderPriorityClass)
				.on("click", selectPriorityClass)
				.transition().duration(500)
				.style("opacity", 1)
			nodeListWrapper.style("display", "");
		} else {
			priorityClassDiv.classed("selectList", false).classed("wide", false);
			priorityClassDiv.html("").append("h4").text("No Priorities");
			nodeListWrapper.style("display", "None");
		}
	}
	nodeListButtons.append("div").classed("blockButton", true)
		.text("Select All")
		.on("click", () => {
			if(selectedPriorityClass < 0) return;
			priorityClasses[selectedPriorityClass].nodes.clear();
			for(let i = 0; i < graph.nodes.length; ++i) {
				priorityClasses[selectedPriorityClass].nodes.add(i);
			}
			renderNodeList();
			renderPriorityClasses();
			createNodeMarkers();
		});
	nodeListButtons.append("div").classed("blockButton", true)
		.text("Unselect All")
		.on("click", () => {
			if(selectedPriorityClass < 0) return;
			priorityClasses[selectedPriorityClass].nodes.clear();
			renderNodeList();
			renderPriorityClasses();
			createNodeMarkers();
		});
	let nodeListItems = nodeList.selectAll("div")
		.data(graph.nodes).join("div")
		.text((_, i) => "Node #"+i)
		.on("click", selectNode)
		.on("mouseover", (_, i) => {
			let icon = nodeMarkers[i]._icon;
			if(icon) icon.children[0].classList.add("hover");
		})
		.on("mouseout", (_, i) => {
			let icon = nodeMarkers[i]._icon;
			if(icon) icon.children[0].classList.remove("hover");
		});
	function selectNode(d, i) {
		if(selectedPriorityClass < 0) return;
		if(priorityClasses[selectedPriorityClass].nodes.has(i)) {
			priorityClasses[selectedPriorityClass].nodes.delete(i);
		} else {
			priorityClasses[selectedPriorityClass].nodes.add(i);
		}
		renderNodeList();
		renderPriorityClasses();
		createNodeMarkers();
	}
	function renderNodeList() {
		let nodes = priorityClasses[selectedPriorityClass].nodes;
		nodeListItems.attr("class", (_, i) => {
			if(nodes.has(i)) return "currentIndex";
			else return "";
		});
	}
	let nodeMarkers = null;
	let nodeMarkerLayer = null;
	let createNodeMarkers = () => {
		if(nodeMarkerLayer) nodeMarkerLayer.remove();
		if(selectedPriorityClass < 0) return;
		nodeMarkers = graph.nodes.map((node, i) => {
			let pri = priorityClasses[selectedPriorityClass].nodes.has(i);
			let m = L.marker(node.latlng, {
				icon: getDescriptionIcon(
					"#"+i,//+" "+(pri > 0 ? "<b>["+pri+"]</b>" : ""),
					pri),
				pane: "resources",
			});
			m.on("click", () => selectNode(node, i));
			return m;
		});
		nodeMarkerLayer = L.featureGroup(nodeMarkers);
		nodeMarkerLayer.addTo(Map);
	};
	selectPriorityClass(null, -1);

	graph.contextMenuListener = (event, menu) => {
		// This must be the first command, because mouse state will change later
		let newTeam = getMouseTeam(event);
		if(teams.length < 10) {
			menu.append("div").text("Add Team Here")
				.on("click", () => {
					selectTab(1);
					teams.push(newTeam);
					selectTeam(null, teams.length - 1);
				})
		}
		if(selectedTeam >= 0) {
			menu.append("div").text("Move Team Here")
				.on("click", () => {
					teams[selectedTeam] = newTeam;
					selectTeam(null, selectedTeam);
				})
		}
	};

	cleanUp = () => {
		if(nodeMarkerLayer) nodeMarkerLayer.remove();
		if(teamMarkerLayer) teamMarkerLayer.remove();
		graph.contextMenuListener = null;
	}
	function getCurrentRequest() {
		horizonValue = parseInt(horizon.property("value"));
		if(isNaN(horizonValue)) {
			errorDiv.text("Invalid horizon");
			return null;
		}
		let request = {
			policyConfig: {
				horizon: horizonValue,
			}
		};
		request.priorities = priorityClasses.filter(a => a.nodes.size > 0).map(a => {
			return {
				nodes: [...a.nodes],
				maxmin: a.maxmin
			};
		});
		if(teams.length > 0) {
			request.teams = teams;
			request.teamType = selectedTeamType;
		}
		request.benchmark = benchmark_enabled;
		return request;
	}
	div.append("div").classed("blockButton", true)
		.text("Generate Policy")
		.on("click", () => {
			request = getCurrentRequest();
			console.log("Request:", request);
			if (request) {
				cleanUp();
				requestNewPolicy(div, graph, request);
			}
		});
	if (lastRequestedPolicy) {
		div.append("div").classed("blockButton", true)
			.text("Re-request Last")
			.on("click", () => {
				requestNewPolicy(...lastRequestedPolicy);
			});
	}
	let experimentDiv = div.append("div");
	experimentDiv.resetDiv = (previousName) => {
		if (!previousName) {
			previousName = graph.name+" Exp";
		}
		experimentDiv.html("");
		experimentDiv.append("hr");
		experimentDiv.append("h3").text("Save this configuration");
		let experimentName = createTextInput(experimentDiv, "Experiment Name", previousName);
		experimentDiv.append("div").classed("blockButton", true)
			.text("Save as Experiment")
			.on("click", () => {
				request = getCurrentRequest();
				request.name = experimentName.property("value");
				console.log("Save Request:", request);
				if (request) {
					saveAsExperiment(experimentDiv, graph, request);
				}
			});
	};
	experimentDiv.resetDiv();
}

function selectPrioritizedNode(div, graph){
	div.html("");
	div.style("opacity", 0).transition().duration(500).style("opacity", 1);
	let horizon = createTextInput(div, "Horizon", defaultHorizon);
	let errorDiv = div.append("p").text("")
		.style("color", "red");
	let algorithms = [
		{ name: "S3P", value: "s3p" },
		{ name: "Arpali Dummy", value: "arpali_dummy" },
		{ name: "Gol Dummy", value: "gol_dummy" },
		{ name: "Custom MaxMin", value: "custom_maxmin" },
		{ name: "Custom MinMin", value: "custom_minmin" },
		{ name: "Greedy", value: "greedy" },
		{ name: "Average", value: "average" },
	];
	let selectedAlgo = algorithms.length - 1;
	let algoDiv = div.append("div");
	let algoDivs = algoDiv.selectAll("div").data(algorithms).join("div")
		.classed("customCheckbox", true)
		.classed("radio", true);
	algoDivs.append("input")
		.attr("type", "radio")
		.attr("name", "algos")
		.attr("id", d => "algo-"+d.value)
		.attr("checked", (_,i) => i == selectedAlgo)
		.on("change", (_, i) => selectedAlgo = i);
	algoDivs.append("label")
		.text(d => "Use "+d.name)
		.attr("for", d => "algo-"+d.value);
	// ids of the prioritized node
	let priorities = {};
	let li;
	let updateList = () => {
		li.attr("class", (_, i) => {
			if(priorities[i] > 0) return "currentIndex";
			else return "";
		})
		.html((_, i) => {
			let name = "Node #"+i;
			if(priorities[i] > 0) name = "<b>["+priorities[i]+"]</b> "+name
			return name;
		})
	};
	let markers = null;
	let markerLayer = null;
	let createMarkers = () => {
		if(markerLayer) markerLayer.remove();
		markers = graph.nodes.map((node, i) => {
			let pri = priorities[i];
			let m = L.marker(node.latlng, {
				icon: getDescriptionIcon(
					"#"+i+" "+(pri > 0 ? "Prioritized <b>["+pri+"]</b>" : "Normal"),
					pri),
				pane: "resources",
			});
			m.on("click", () => selectFun(node, i));
			return m;
		});
		markerLayer = L.featureGroup(markers);
		markerLayer.addTo(Map);
	};
	let selectFun = (_, i) => {
		if(i in priorities) {
			let count = Object.values(priorities)
				.filter(p => p === priorities[i]).length;
			if(count > 1) {
				++priorities[i];
			} else {
				delete priorities[i];
			}
		} else {
			priorities[i] = 1;
		}
		console.log(priorities);
		updateList();
		createMarkers();
	};
	let list = div.append("div")
		.classed("selectList", true);
	li = list.selectAll("div").data(graph.nodes).join("div")
		.text((_, i) => "Node #"+i)
		.on("click", selectFun)
	/*
		.on("mouseover", (_, i) => {
			let icon = markers[i]._icon;
			if(icon) icon.children[0].classList.add("hover");
		})
		.on("mouseout", (_, i) => {
			let icon = markers[i]._icon;
			if(icon) icon.children[0].classList.remove("hover");
		});
		*/
	updateList();
	createMarkers();
	cleanUp = () => {
		if(markerLayer) markerLayer.remove();
	}
	div.append("p").text(`Info`);
	div.append("div").classed("blockButton", true)
		.text("Generate Policy")
		.on("click", () => {
			horizonValue = parseInt(horizon.property("value"));
			if(isNaN(horizonValue)) {
				errorDiv.text("Invalid horizon");
				return;
			}
			cleanUp();
			let prioritized = [];
			for(let node in priorities) {
				let p = priorities[node] - 1;
				if(!prioritized[p]) prioritized[p] = [];
				prioritized[p].push(node);
			}
			console.log("Prioritized:",prioritized);
			requestNewPolicy(div, graph, {
				prioritized: prioritized,
				algo: algorithms[selectedAlgo].value,
				conf: {
					horizon: horizonValue,
				}
			});
		});
}

function selectPolicyOptions(div, graph){
	div.html("");
	div.style("opacity", 0).transition().duration(500).style("opacity", 1);
	let horizon = createTextInput(div, "Horizon", defaultHorizon);
	let shortSighted = false;
	createCheckbox(div, "Short-sighted", val => {
		shortSighted = val;
	});
	let errorDiv = div.append("p").text("")
		.style("color", "red");
	div.append("div").classed("blockButton", true)
		.text("Generate Policy")
		.on("click", () => {
			horizonValue = parseInt(horizon.property("value"));
			if(isNaN(horizonValue)) {
				errorDiv.text("Invalid horizon");
				return;
			}
			requestNewPolicy(div, graph, {
				conf: {
					horizon: horizonValue,
				},
				shortsighted: shortSighted,
			});
		});
}

function selectPolicyView(div, graph) {
	graph.emptyState();
	graph.rerender();
	policySettings(div, graph);
	return;

	function createTrivialPolicy() {
		//this.infoText = "A trivial policy has been generated.";
		let policy = trivialPolicy(graph);
		policyView = new TrivialPolicyView(graph, div, policy);
	}
	div.html("");
	div.append("div").classed("blockButton", true)
		.text("No Prioritization")
		.on("click", () => {
			selectPolicyOptions(div, graph);
		});
	div.append("div").classed("blockButton", true)
		.text("Prioritize")
		.on("click", () => {
			selectPrioritizedNode(div, graph);
		});
	div.append("div").classed("blockButton", true)
		.text("New UI")
		.on("click", () => {
			policySettings(div, graph);
		});
	div.append("p")
		.text(`Deprecated buttons`);
	div.append("div").classed("blockButton", true)
		.text("Request Policy From Server")
		.on("click", () => getPolicy(div, graph));
	div.append("div").classed("blockButton", true)
		.text("Synthesize Trivial Policy")
		.on("click", createTrivialPolicy);
}



class InteractivePolicyView {
	/**
	 * Takes a graph and div.
	 * div must be a d3 selection
	 */
	constructor(_graph, div, policy, options={}) {
		this.div = div;
		this.graph = _graph;
		this.policy = policy;
		this.options = options;
		Object.assign(this, options);
		this.teamMarkers = [];
		this.infoEnabled = true;
		this.policyNavigator();
	}
	setNext(i) {
		this.policy.setNext(i);
		this.createMarkerLayer();
		//this.policyNavigator();
	}
	teamOnMouseOver(event) {
		let index = event.target.data;
		let team = this.policy.getCurrentTeams()[index];
		let next = this.policy.end ? null : this.policy.getNextTeams()[index];
		let div = d3.select(Tooltip.div);
		div.html("");
		div.append("b").text(`Team #${index+1}`);
		let info = event.target.tooltipInfo;
		for(let text of info) {
			div.append("p").text(text);
		}
		Tooltip.show(event.originalEvent);
	}

	renderTeam(i, currentTeam, nextTeam) {
		let color = TeamColors[i];
		let node = this.policy.teamNodes[currentTeam.node];
		let nextNode = null;
		let position = node;
		let info = [];
		if(currentTeam.target != null) {
			nextNode = this.policy.teamNodes[currentTeam.target];
      let travelTime = this.policy.travelTimes[currentTeam.node][currentTeam.target];
			let percent = currentTeam.time / travelTime;
			position = [
				node[0]*(1-percent) + nextNode[0]*percent,
				node[1]*(1-percent) + nextNode[1]*percent
			];
			if(currentTeam.node < this.graph.nodes.length) {
				info.push("Was previously at node #" + currentTeam.node);
			}
			info.push("Moving to #" + currentTeam.target + ", "+
				(currentTeam.travelTime-currentTeam.time) +" seconds remaining.");
		} else if(nextTeam) {
			let nextNodeId = nextTeam.target != null ? nextTeam.target : nextTeam.node;
			nextNode = this.policy.teamNodes[nextNodeId];
			if(currentTeam.node < this.graph.nodes.length) {
				info.push("Currently at node #" + currentTeam.node);
			}
			info.push("Will move to #" + nextNodeId + ".");
		} else if(currentTeam.node < this.graph.nodes.length) {
			info.push("Currently at node #" + currentTeam.node);
		}
		if(Settings.renderTeamArrows && nextNode != null) {
			let line = L.polyline([node, nextNode], {
				color: color,
				dashArray: "10, 10",
				weight: 2,
				smoothFactor: 1,
				pane: "teamArrows"
			});
			let deco = L.polylineDecorator( line, {
				patterns: [
					{offset: 30, repeat: 30, symbol: L.Symbol.arrowHead({
						pixelSize: 20,
						pathOptions: {
							color: color, fillOpacity: 1, weight: 0,
							pane: "teamArrows"
						},
					})},
				],
			});
			this.markers.push(line);
			this.markers.push(deco);
		}
		if(this.teamMarkers[i]) {
			let m = this.teamMarkers[i];
			m._icon.style.transition = '0.6s';
			m.setLatLng(position);
			m.tooltipInfo = info;
		} else {
			let m = L.marker(position, {
				icon: TeamIcons[i],
				color: color,
				pane: "teams",
			});
			m.data = i;
			m.tooltipInfo = info;
			m.on("mouseover", this.teamOnMouseOver.bind(this));
			m.on("mouseout", Tooltip.hide.bind(Tooltip));
			m.addTo(Map);
			this.teamMarkers[i] = m;
			m._icon.addEventListener("transitionend", () => {
				m._icon.style.transition = '';
			});
		}
	}

	createMarkerLayer() {
		// remove the old one if any
		if(this.markerLayer) this.markerLayer.remove();
		this.markers = [];
		if(this.policy.end) {
			// just render teams and return
			if(this.policy.teams) {
				let currentTeams = this.policy.getCurrentTeams();
				for(let i=0; i<currentTeams.length; ++i) {
					let currentTeam = currentTeams[i];
					this.renderTeam(i, currentTeam, null);
				}
			}
			this.markerLayer = L.featureGroup(this.markers);
			this.markerLayer.addTo(Map);
			return;
		}
		if(Settings.renderNextStateInfo) {
			let currentState = this.policy.getCurrentState();
			let nextState = this.policy.getNextState();
			for(let i=0; i<currentState.length; ++i) {
				if(currentState[i] != nextState[i]) {
					let node = this.graph.nodes[i];
					let desc = nextState[i];
					let m = L.marker(node.latlng, {
						icon: getDescriptionIcon("#"+i+" "+desc, node.status < 0),
						pane: "resources",
					});
					this.markers.push(m);
				}
			}
		}
		if(this.policy.teams) {
			let currentTeams = this.policy.getCurrentTeams();
			let nextTeams = this.policy.getNextTeams();
			for(let i=0; i<currentTeams.length; ++i) {
				let currentTeam = currentTeams[i];
				let nextTeam = nextTeams[i];
				this.renderTeam(i, currentTeam, nextTeam);
			}
		}
		this.markerLayer = L.featureGroup(this.markers);
		this.markerLayer.addTo(Map);
	}
	policyNavigator() {
		this.createMarkerLayer();
		this.div.html("");
		let buttonDiv = this.div.append("div").classed("policyControls", true);
		let prev = buttonDiv.append("div").classed("blockButton", true)
			.text("Previous Step");
		let next = buttonDiv.append("div").classed("blockButton", true)
			.text("Next Step");
		let infoButton = buttonDiv.append("div").classed("blockButton", true)
			.text(this.infoEnabled ? "Hide Info" : "Show Info");

		let infoDiv = this.div.append("div");
		if(this.prelude) this.prelude(infoDiv);
		let infoList = infoDiv.append("ul");
		if(this.policy.totalTime) {
      let totalTime = Math.round(100000*this.policy.totalTime)/100000;
      let text = "Elapsed time: "+totalTime;
      if (this.policy.generationTime) {
        let generationTime = Math.round(100000*this.policy.generationTime)/100000;
        text += " (Generation: " + generationTime + ")";
      }
			infoList.append("li").text(text);
		}
		infoList.append("li")
			.text("State/States: "+this.policy.state+" / "+this.policy.states.length);
		if(this.policy.values) {
			infoList.append("li").text("Value: "+Math.min(...Object.values(this.policy.values[0])));
		}
		if(this.policy.times) {
			let time = this.policy.times[this.policy.state];
			infoList.append("li").text("Time: "+time);
		}
		if(!this.infoEnabled) {
			infoDiv.style("display", "None");
		}

		infoButton.on("click", () => {
			this.infoEnabled = !this.infoEnabled;
			if(this.infoEnabled) {
				infoButton.text("Hide Info");
				infoDiv.style("display", "");
			} else {
				infoButton.text("Show Info");
				infoDiv.style("display", "None");
			}
		});

		if(this.policy.previousStates.length>0) {
			prev.on("click", () => {
				this.policy.previousState();
				this.policyNavigator(); // refresh
			});
		} else {
			prev.classed("disabled", true);
		}

		if(this.policy.nextStateAvailable()) {
			/*
			buttonDiv.append("div").classed("blockButton", true)
				.text("krink")
				.on("click", krink)
			buttonDiv.append("div").classed("blockButton", true)
				.text("grink")
				.on("click", getGraphDiagram)
			*/
			// select box for action
			var innerDiv;
			const wrapperDiv = this.div.append("div")
				.style("margin", "1em")
				.attr("class","CustomSelect");
			wrapperDiv.append("div")
				.attr("class", "CustomSelectHead")
				.html(this.policy.describeAction())
				.on("click", function(_) {
					innerDiv.classList.toggle("open");
				}).node();
			const ul = wrapperDiv.append("div").attr("class", "CustomSelectList").append("div");
			innerDiv = ul.node();

			ul.selectAll("div")
				.data(Object.keys(this.policy.actions)).join("div")
				.attr("class", "CustomSelectElement")
				.html((i) => this.policy.describeAction(i))
				.on("click", (i) => {
					this.policy.setAction(i, 0);
					this.policyNavigator();
				});

			next.on("click", () => {
				this.policy.nextState();
				this.policyNavigator(); // refresh
			});
			this.div.append("p").text("The following transitions are possible:");
			let transitionList = this.div.append("div")
				.classed("selectList", true)
				.classed("wide", true);
			let updateLi = (li) => {
				li.attr("class", (_, i) => {
					if(i == this.policy.next) return "selected";
					else return "";
				})
			}
			let li = transitionList.selectAll("div")
				.data(this.policy.action).join("div")
				.html(this.policy.describeTransition.bind(this.policy))
				.on("click", (_, i) => {
					this.setNext(i);
					updateLi(li);
				});
			updateLi(li);
			/*
			let p = this.policy.successProbability(this.policy.index+1);
			this.div.append("p").text("Success Probability for next step: "+
				p.toFixed(3));
			*/
		} else {
			next.classed("disabled", true);
			this.div.append("p")
				.text("No more actions are available.");
		}
	}
	updateMode() {
		if(this.markerLayer) this.markerLayer.remove();
		this.div.html("");
		this.div.append("h3").text("Updating Graph");
		let div = this.div.append("div");
		div.append("p").text("You are updating the graph right now.");
		div.append("p").text(`
			When you finish updating, click "Done" button to
			update the policy.`);
		div.append("div").classed("blockButton", true)
			.text("Done")
			.on("click", () => {
				div.html("");
				addSpinnerDiv(div).append("p")
					.text("Waiting response from server...");
				requestPolicy(this.graph, this.options).then(response => {
					let policy = JSON.parse(response);
					this.policy.update(policy);
					this.policyNavigator();
				}).catch(error => {
					div.html("");
					div.selectAll("p").remove();
					div.append("b").style("color", "red")
						.text("Failed to get updated policy");
					div.append("p").style("color", "red")
						.text(error);
					div.append("div").classed("blockButton", true)
						.text("Go back")
						.on("click", this.updateMode.bind(this));
				});
			});
		div.append("div").classed("blockButton", true)
			.text("Cancel")
			.on("click", () => {
				this.graph.cancelUpdate();
				this.policyNavigator();
			});
	}
	nodeOnInfo(node, div) {
		if(node.status > 0) return;
		let p = this.policy.energizationProbabilities(node.index);
		let index = p.lastIndexOf(0);
		p = p.slice(index, p.indexOf(p[p.length-1])+1);
		div = d3.select(div);
		div.append("div").text("Probability of energization in...");
		div.append("ul").selectAll("li").data(p).join("li")
			.text((d,i) => `${i==p.length-1 ? (index+i)+"+" : i+index} steps: `+(Math.round(10000*d)/10000));
	}
	destroy() {
		if(this.markerLayer) this.markerLayer.remove();
		if(this.teamMarkers) {
			for(let m of this.teamMarkers) {
				m.remove();
			}
		}
	}
}

function removePolicy() {
	if(cleanUp) cleanUp();
	if(policyView && policyView.destroy) policyView.destroy();
	policyView = null;
}
