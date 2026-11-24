export const brand = 'tomato';
export const sizes = { small: 4, large: 4 * 4 };
export const derived = `${brand}-${sizes.large}`;
export const bad = window.innerWidth;
const local = 'not exported';
export { local };
