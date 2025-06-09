// TODO: RIIR!
export function start_animation(animation_step) {
  return new Promise((resolve) => {
    const update = () => {
      if (!animation_step()) {
        window.requestAnimationFrame(update);
      } else {
        resolve();
      }
    };
    update();
  });
}
