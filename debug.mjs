import mod from './src/wasm/bms-core.js';

const wasm = await (mod.default || mod)();

const p = (a) => [[0, 0, 0], a, 0]; // ψ₁(arg)
const z = [0, 0, 0]; // ψ₀(0) = 1

// Ω = ψ₁(0)
const o1 = p(z);
// Ω^Ω = ψ₁(ψ₁(0))
const o2 = p(p(z));
// Ω^{Ω^Ω} = ψ₁(ψ₁(ψ₁(0)))
const o3 = p(p(p(z)));

console.log('Ω =', JSON.stringify(wasm.decomposePower(o1)));
console.log('Ω^Ω =', JSON.stringify(wasm.decomposePower(o2)));
console.log('Ω^{Ω^Ω} =', JSON.stringify(wasm.decomposePower(o3)));
console.log('t(Ω^{Ω^Ω}) =', JSON.stringify(wasm.computeT(o3)));
console.log('t(Ω^Ω) =', JSON.stringify(wasm.computeT(o2)));
console.log('t(Ω) =', JSON.stringify(wasm.computeT(o1)));
