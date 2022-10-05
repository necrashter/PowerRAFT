
function trivialPolicy(grph) {
	// policy starts with null, because it represents the initial
	// state, before any actions are taken
	let steps = [null];
	// we can only activate unknown nodes
	let nodesToActivate = grph.nodes.filter(node => node.status == 0);
	while(nodesToActivate.length > 0) {
		let remaining = nodesToActivate.length;
		let nextNodes = Math.min(remaining, Math.ceil(Math.random()*3));
		steps.push({
			nodes: nodesToActivate.splice(0, nextNodes),
		});
	}
	return new TrivialPolicy(grph, steps);
}

class TrivialPolicy {
	constructor(_graph, steps) {
		this.graph = _graph;
		this.steps = steps;
		this.index = 0;
	}
	goTo(index) {
		this.index = index;
		for(let i=1; i<this.steps.length; ++i) {
			let nodes = this.steps[i].nodes;
			if(i <= index) {
				nodes.forEach(node => node.status = 1);
			} else {
				nodes.forEach(node => node.status = 0);
			}
		}
	}
	/**
	 * Calculates the probability of failure of the specified step, relative
	 * to current step.
	 */
	successProbability(index) {
		let p = 1;
		for(let i = this.index+1; i<=index; ++i) {
			let nodes = this.steps[i].nodes;
			nodes.forEach(node => {
				p *= (1-node.pf);
			});
		}
		return p;
	}
	getName(d) {
		if(d==null) return "Initial Status";
		else {
			return "Activate Nodes "+
				(d.nodes.map(node => "#"+node.index).join(", "));
		}
	}
}

class TrivialPolicyView {
	/**
	 * Takes a graph and div.
	 * div must be a d3 selection
	 */
	constructor(_graph, div, policy) {
		this.div = div;
		this.graph = _graph;
		this.setPolicy(policy);
	}
	setPolicy(policy) {
		this.policy = policy;
		this.createMarkerLayer();
		this.policyNavigator();
	}
	goToPolicyStep(index) {
		this.policy.goTo(index);
		this.graph.rerender();
		this.policyNavigator();
	}
	createMarkerLayer() {
		this.markers = [];
		for(let i=1; i<this.policy.steps.length; ++i) {
			let nodes = this.policy.steps[i].nodes;
			nodes.forEach(node => {
				let m = L.marker(node.latlng, {
					icon: getNumberIcon(i),
					pane: "resources"
				});
				m.on("click", () => this.goToPolicyStep(i));
				this.markers.push(m);
			});
		}
		this.markerLayer = L.layerGroup(this.markers);
		//this.markerLayer.addTo(Map);
	}
	policyNavigator() {
		this.div.html("");
		if(this.infoText) this.div.append("p").text(this.infoText);
		this.div.append("p")
			.text(`You can use the buttons or 
				click on a step to jump directly to it.`);
		let buttonDiv = this.div.append("div").classed("policyControls", true);
		let prev = buttonDiv.append("div").classed("blockButton", true)
			.text("Previous Step");
		let next = buttonDiv.append("div").classed("blockButton", true)
			.text("Next Step");
		if(this.policy.index>0) {
			prev.on("click", () => {
				this.goToPolicyStep(this.policy.index-1);
			});
		} else {
			prev.classed("disabled", true);
		}
		if(this.policy.index < this.policy.steps.length -1) {
			next.on("click", () => {
				this.goToPolicyStep(this.policy.index+1);
			});
			let p = this.policy.successProbability(this.policy.index+1);
			this.div.append("p").text("Success Probability for next step: "+
				p.toFixed(3));
		} else {
			next.classed("disabled", true);
		}
		// lists policy steps
		let stepList = this.div.append("div").classed("selectList", true);
		stepList.selectAll("div").data(this.policy.steps).join("div")
			.text(this.policy.getName)
			.attr("class", (_, i) => {
				if(i > this.policy.index) return "disabled";
				else if(i == this.policy.index) return "currentIndex";
				else return "";
			})
			.on("click", (_, i) => {
				this.goToPolicyStep(i);
			});
		{
			let div = this.div.append("div").classed("customCheckbox", true);
			let checkbox = div.append("input")
				.attr("id", "showNums")
				.attr("type", "checkbox")
				.on("change", () => {
					if(checkbox.node().checked)
						this.markerLayer.addTo(Map);
					else
						this.markerLayer.remove();
				})
				.property("checked", this.markerLayer._map != null);
			div.append("label")
				.attr("for", "showNums")
				.text("Show Numbers on Map");
		}
		if(this.policy.index == this.policy.steps.length-1) {
			let endDiv = this.div.append("div");
			endDiv.append("h2").text("Congratulations!");
			endDiv.append("p")
				.text("You have reached the end of the policy.");
			endDiv.style("opacity", 0)
				.transition().duration(500)
				.style("opacity", 1);
		}
	}
	destroy() {
		if(this.markerLayer) this.markerLayer.remove();
	}
}

