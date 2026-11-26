// main.js - Entry point: register the scroll scenes, wire up the UI, start the frame loop.
import { start } from './engine.js';
import { heroScene, tourScene, raceScene, galleryScene, marqueeScene, stepsScene, chromeScene, cursorScene } from './scenes.js';
import { initTheme, initMenu, initFaq, initReveal, initScan, initPointerFx, initRelease, initEasterEgg } from './ui.js';

const hero = heroScene();
tourScene();
raceScene();
galleryScene();
marqueeScene();
stepsScene();
chromeScene();
cursorScene();

initTheme(() => hero.retheme());
initMenu();
initFaq();
initReveal();
initScan();
initPointerFx();
initRelease();
initEasterEgg();

start();
