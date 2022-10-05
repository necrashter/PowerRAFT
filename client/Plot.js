
function horizontalBarPlot(data, width, height, options={}) {
	let margin = {top: 20, right: 40, bottom: 40, left: 90};
	if(options.margin) Object.assign(margin, options.margin);
	// effective width and height
	let ew = width - margin.right - margin.left;
	let eh = height - margin.top - margin.bottom;
	let SVG = d3.create("svg")
		.attr("width", width )
		.attr("height", height )
	let svg = SVG.append("g")
		.attr("transform",
          "translate(" + margin.left + "," + margin.top + ")");
	var x = d3.scaleLinear()
		.domain([0, d3.max(data.map(d => d.value))])
		.range([ 0, ew]);
	svg.append("g")
		.attr("transform", "translate(0," + eh + ")")
		.call(d3.axisBottom(x))
		.selectAll("text")
		.attr("font-size", "16px")
		//.attr("transform", "translate(-10,0)rotate(-45)")
		//.style("text-anchor", "end");
	var y = d3.scaleBand()
		.range([ 0, eh ])
		.domain(data.map((d) => d.name))
		.padding(.1);
	svg.append("g")
		.call(d3.axisLeft(y))
		.selectAll("text")
		.attr("font-size", "16px");
	svg.selectAll("myRect")
		.data(data)
		.enter()
		.append("rect")
		.attr("x", x(0) )
		.attr("y", (d) => y(d.name) )
		.attr("width", 0)
		.attr("height", y.bandwidth() )
		.attr("fill", "#14fdce")
		.on("mouseover", (d) => {
			Tooltip.div.innerHTML = d.name + " " + d.value;
			Tooltip.show();
		})
		.on("mouseout", Tooltip.hide)
		//.on("click", (d) => console.log(d))
		.transition().duration(1000)
		.attr("width", (d) => x(d.value))
	svg.append("g")
		.attr("fill", "#022011")
		.attr("text-anchor", "end")
		.attr("font-family", "sans-serif")
		.attr("font-size", 16)
    .selectAll("text")
    .data(data)
    .join("text")
		.attr("x", d => x(d.value))
		.attr("y", (d) => y(d.name) + y.bandwidth() / 2)
		.attr("dy", "0.35em")
		.attr("dx", -4)
		.text(d => d.value)
    .call(text => text.filter(d => x(d.value) - x(0) < 20) // short bars
		.attr("dx", +4)
		.attr("fill", "#14fdce")
		.attr("text-anchor", "start"));
	return SVG.node();
}

/**
 * data = [ {name: str , value: int } ]
 */
function donutChart(data, width, height, options = {}) {
	data = data.filter(d => d.value > 0);
	let pie = d3.pie()
		.padAngle(0.005)
		.sort(null)
		.value(d => d.value)
	const radius = (Math.min(width, height) / 2) * 0.75;
	let arc =  d3.arc().innerRadius(radius * 0.6).outerRadius(radius - 1);
	let arc2 =  d3.arc().innerRadius(radius * 0.8).outerRadius(radius - 1);
	let arcLabel = d3.arc().innerRadius(radius*1.2).outerRadius(radius*1.2);
	let color = d3.scaleOrdinal()
		.domain(data.map(d => d.name))
		.range(d3.quantize(t => d3.interpolateSpectral(t * 0.8 + 0.1), data.length).reverse())
	let customLabel = (d) => {
		let centroid = arcLabel.centroid(d);
		return [centroid[0]*1.1, centroid[1]];
	}


	const arcs = pie(data);

	const svg = d3.create("svg")
		.attr("viewBox", [-width / 2, -height / 2, width, height]);
	const labels = svg.append("g");

	labels.selectAll("line").data(arcs)
		.join("line")
		.attr("x1", d => arc.centroid(d)[0])
		.attr("y1", d => arc.centroid(d)[1])
		.attr("x2", d => {
			let x = customLabel(d)[0];
			x += x < 0 ? 5 : -5;
			return x;
		})
		.attr("y2", d => customLabel(d)[1])
		.attr("stroke", "currentColor")
		.attr("stroke-width", "2")
	path = svg.selectAll("path")
		.data(arcs)
		.join("path")
		.attr("fill", "currentColor")
		.attr("d", arc2)
	path.append("title")
		.text(d => `${d.data.name}: ${d.data.value.toLocaleString()}`);
	function arcAnim(d) {
		var interpolateStart = d3.interpolate(0, d.startAngle);
		var interpolateEnd = d3.interpolate(0, d.endAngle);
		return (t) => {
			d.startAngle = interpolateStart(t);
			d.endAngle = interpolateEnd(t);
			return arc(d);
		}
	}
	path.transition().duration(1000)
		.attrTween("fill", function(d) {
			return d3.interpolateRgb(this.getAttribute("fill"), color(d.data.name));
		})
		.attrTween("d", arcAnim)

	labels.append("g")
		.attr("font-family", "sans-serif")
		.attr("font-size", 16)
		.attr("fill", "currentColor")
		.selectAll("text")
		.data(arcs)
		.join("text")
		.attr("text-anchor", d => customLabel(d)[0] < 0 ? "end" : "start")
		.attr("transform", d => `translate(${customLabel(d)})`)
		.call(text => text.append("tspan")
			.text(d => d.data.name))
	svg.append("g")
		.attr("font-family", "sans-serif")
		.attr("font-size", 16)
		.attr("text-anchor", "middle")
		//.attr("fill", "currentColor")
		.selectAll("text")
		.data(arcs)
		.join("text")
		.attr("transform", d => `translate(${arc.centroid(d)})`)
		.call(text => text.filter(d => (d.endAngle - d.startAngle) > 0.25).append("tspan")
			.attr("x", 0)
			.attr("y", "0.7em")
			.attr("fill-opacity", 0.7)
			.text(d => d.data.value.toLocaleString()))
		.style("opacity", "0")
		.transition().duration(1000)
		.style("opacity", "1");

	labels
		.style("opacity", "0")
		.transition().duration(1000)
		.style("opacity", "1");

	return svg.node();
}


function createNodeStats() {
	var labels = ["Damaged", "Unknown", "Energized"];
	var data = [0, 0, 0];
	graph.nodes.forEach(branch => {
		data[branch.status+1] += 1;
	});
	let domain = [0, d3.max(data)];
	let x = d3.scaleLinear()
		.domain(domain)
		.range([0, 80]);
	const div =  d3.create("div").classed("barWrap", true);

	let join = div.selectAll("div").data(data).join("div")
		.classed("horizontalBarWrap", true);
	let bar = join.append("div")
		.classed("horizontalBar", true)
		.style("width", "0px")
	bar.transition().duration(1500)
		.style("width", d => `${x(d)}%`)
	bar.append("p").text((d) => d)
	join.append("div")
		.text((_,i) => labels[i]);
	return div.node();
}
