

function addNode(pos, pf = 0.25) {
	let node = {
		latlng: pos,
		pf,
		status: 0,
		index: graph.nodes.length,
	};
	graph.nodes.push(node);
	return node;
}


function veclerp(v1, v2, fac) {
	return [
		(v1[0] * (1-fac)) + (v2[0] * fac),
		(v1[1] * (1-fac)) + (v2[1] * fac),
	];
}


Graph.generate = {
	"wscc": function wscc(internal = 1, external = 1) {
		const downVec = [-1, 0];
		const rightVec = [Math.sin(Math.PI / 6), Math.cos(Math.PI / 6)];
		const leftVec = [Math.sin(Math.PI / 6), -Math.cos(Math.PI / 6)];
		const d = 0.016666666666666666;
		const r = ((internal + 1) * d) / (2 * Math.sin(Math.PI / 3));

		const center = graph.map.getCenter();
		const dbus = [
			downVec[0] * r + center.lat,
			downVec[1] * r + center.lng,
		];
		const rbus = [
			rightVec[0] * r + center.lat,
			rightVec[1] * r + center.lng,
		];
		const lbus = [
			leftVec[0] * r + center.lat,
			leftVec[1] * r + center.lng,
		];

		graph.nodes = [];
		graph.branches = [];
		graph.externalBranches = [];
		graph.resources = [];

		const addMiddleNodes = (apos, bpos, count) => {
			for (let i = 1; i <= count; ++i) {
				const fac = i / (count + 1);
				const pos = veclerp(apos, bpos, fac);
				addNode(pos);
			}
		};

		const addExternalNodes = (apos) => {
			for (let i = 0; i <= external; ++i) {
			}
		};

		// Add corner and internal edges
		addNode(lbus);
		addMiddleNodes(lbus, rbus, internal);
		addNode(rbus);
		addMiddleNodes(rbus, dbus, internal);
		addNode(dbus);
		addMiddleNodes(dbus, lbus, internal);

		// Add sources
		const lsource = [
			lbus[0] + (leftVec[0] * d * (external+1)),
			lbus[1] + (leftVec[1] * d * (external+1)),
		];
		const rsource = [
			rbus[0] + (rightVec[0] * d * (external+1)),
			rbus[1] + (rightVec[1] * d * (external+1)),
		];
		const dsource = [
			dbus[0] + (downVec[0] * d * (external+1)),
			dbus[1] + (downVec[1] * d * (external+1)),
		];

		graph.resources.push({type: null, latlng: lsource});
		graph.resources.push({type: null, latlng: rsource});
		graph.resources.push({type: null, latlng: dsource});

		// Add external nodes
		addMiddleNodes(lbus, lsource, external);
		addMiddleNodes(rbus, rsource, external);
		addMiddleNodes(dbus, dsource, external);

		// Add internal branches
		const totalInternal = internal*3 + 3;
		for (let i = 0; i < totalInternal; ++i) {
			const other = i + 1 == totalInternal ? 0 : i+1;
			graph.branches.push({nodes: [i, other]});
		}

		// Add external branches
		let exid = totalInternal;
		for (let j = 0; j < 3; ++j) {
			graph.branches.push({nodes: [j*(internal+1), exid]});
			for (let i = 1; i < external; ++i) {
				graph.branches.push({nodes: [exid, ++exid]});
			}
			graph.externalBranches.push({
				source: j,
				node: exid,
				status: 1,
			});
			++exid;
		}

		graph.rerender();
	}
}
