
function getTouchesInfo(touchEvent){
    let { touches } = touchEvent;
    let touchCount = touches.length;
    let centerX = 0;
    let centerY = 0;
    let averageDistance = 0;
    for(let touch of touches){
        let {screenX, screenY} = touch;
        centerX += screenX;
        centerY += screenY;
    }
    centerX /= touchCount;
    centerY /= touchCount;
    for(let touch of touches){
        let {screenX, screenY} = touch;
        let dx = screenX - centerX;
        let dy = screenY - centerY;
        averageDistance += Math.sqrt(dx * dx + dy * dy);
    }
    averageDistance /= touchCount;
    return { centerX, centerY, averageDistance, touchCount };
}

function getTime(){
    return new Date().getTime();
}

export class App {
    constructor(pkg, canvasElement){
        this._oldTouches = [];
        this._previousMouseX = 0;
        this._previousMouseY = 0;
        this._canvas = pkg.get_rust_canvas(canvasElement);
        this._canvas.set_xrange(-10, 10);
        this._canvas.set_yrange(-10, 10);
        canvasElement.addEventListener("mousedown", this.handleMouseDown.bind(this));
        canvasElement.addEventListener("mouseup", this.handleMouseUp.bind(this));
        canvasElement.addEventListener("mousemove", this.handleMouseMove.bind(this));
        canvasElement.addEventListener("touchstart", this.handleTouchStart.bind(this));
        canvasElement.addEventListener("touchmove", this.handleTouchMove.bind(this));
        canvasElement.addEventListener("touchend", this.handleTouchEnd.bind(this));
        
        canvasElement.addEventListener("wheel", this.handleScroll.bind(this));
        this._needsRedraw = true;
        this._idleFrames = 0;
        requestAnimationFrame(() => this.handleFrame());
    }

    _invalidate(){
        this._needsRedraw = true;
    }
    
    _stopAnimation(){

    }

    handleScroll(event) {
        event.preventDefault();
        this._stopAnimation();
        let direction = Math.sign(event.deltaY);
        this._canvas.scale_around(Math.pow(0.95, direction), new Vec2(event.clientX, event.clientY));
        this._invalidate();
    }
    
    handlePinch(x, y, delta) {
        this._stopAnimation();
        this._canvas.scale_around(Math.pow(0.98, delta), new Vec2(x, y));
        this._invalidate();
    }
    
    handleResize() {
        // _canvas.translateOrigin((_platform.width - _oldPlatformWidth) / 2, (_platform.height - _oldPlatformHeight) / 2)
        // _oldPlatformWidth = _platform.width
        // _oldPlatformHeight = _platform.height
        // _stopAnimation
        // _draw
    }
    
    handleTouchStart(event) {
        event.preventDefault();
        let { centerX, centerY, averageDistance, touchCount } = getTouchesInfo(event);
        let time = getTime();
        this._stopAnimation();
        this._oldTouches.push({centerX, centerY, averageDistance, touchCount, time});
    }
    
    handleTouchMove(event) {
        event.preventDefault();
        let { centerX, centerY, averageDistance, touchCount } = getTouchesInfo(event);
        let previous = this._oldTouches[this._oldTouches.length - 1];
        if(previous.touchCount === touchCount) {
            if(averageDistance !== 0 && previous.averageDistance !== 0) {
                this._canvas.scale_around(averageDistance / previous.averageDistance, new Vec2(previous.centerX, previous.centerY));
            }
            this._canvas.translate(new Vec2(centerX - previous.centerX, centerY - previous.centerY));
            this._invalidate();
        }
        let time = getTime();
        this._oldTouches.push({centerX, centerY, averageDistance, touchCount, time});
    }
    
    handleTouchEnd(event) {
        event.preventDefault();
        let { centerX, centerY, averageDistance, touchCount } = getTouchesInfo(event);
        let time = getTime();
        if(touchCount !== 0) {
            this._oldTouches.push({centerX, centerY, averageDistance, touchCount, time})
            return;
        }

        let oldTouches = this._oldTouches;
        this._oldTouches = [];

        // Search for an old touch that was long enough ago that the velocity should be stable
        for(let i = oldTouches.length - 2; i >= 0; i--) {
            // Ignore touches due to a pinch gesture
            if(oldTouches[i].touchCount > 1) {
                return;
            }

            // If we find an old enough touch, maybe do a fling
            if(time - oldTouches[i].time > 0.1 * 1000) {
                this._maybeFling(oldTouches[i], oldTouches[i + 1]);
                return;
            }
        }
    }

    _maybeFling(beforeTouch, afterTouch){
        // let scale = 1 / (afterTouch.time - beforeTouch.time);
        // let vx = (afterTouch.centerX - beforeTouch.centerX) * scale;
        // let vy = (afterTouch.centerY - beforeTouch.centerY) * scale;
        // let speed = Math.sqrt(vx * vx + vy * vy);
        // let duration = Math.log(1 + speed) / 5;
        // let flingDistance = speed * duration / 5; // Divide by 5 since a quintic decay function has an initial slope of 5

        // // Only fling if the speed is fast enough
        // if(speed > 50) {
        //     _startAnimation(.DECAY, duration);
        //     _endOrigin += velocity * (flingDistance / speed);
        // }
    }
    
    handleMouseDown(event) {
        let { clientX : x, clientY : y } = event;
        // this.setCursor(.MOVE);
        this._mouseDown = true;
        this._previousMouseX = x;
        this._previousMouseY = y;
    }
    
    handleMouseMove(event) {
        let { clientX : x, clientY : y, buttons } = event;
        if(buttons > 0){ 
            this._canvas.translate(new Vec2(x - this._previousMouseX, y - this._previousMouseY));
            this._invalidate();
            // this.setCursor(.MOVE);
        }
    
        this._previousMouseX = x;
        this._previousMouseY = y;
    }
    
    handleMouseUp(event) {
        let { clientX : x, clientY : y, buttons } = event;
        console.log(event);
        if(buttons === 0) {
            this._mouseDown = false;
            // this._mouseAction = .NONE
            // this.setCursor(.DEFAULT);
        }
    
        this._previousMouseX = x;
        this._previousMouseY = y;
    }


    handleFrame() {
        requestAnimationFrame(() => this.handleFrame());
        
		// let time = getTime();

		// if _animation != .NONE {
		// 	var t = (time - _startTime) / (_endTime - _startTime)

		// 	# Stop the animation once it's done
		// 	if t > 1 {
		// 		_canvas.setOriginAndScale(_endOrigin.x, _endOrigin.y, _endScale)
		// 		_animation = .NONE
		// 	}

		// 	else {
		// 		# Bend the animation curve for a more pleasant animation
		// 		if _animation == .EASE_IN_OUT {
		// 			t *= t * t * (t * (t * 6 - 15) + 10)
		// 		} else {
		// 			assert(_animation == .DECAY)
		// 			t = 1 - t
		// 			t = 1 - t * t * t * t * t
		// 		}

		// 		# Animate both origin and scale
		// 		_canvas.setOriginAndScale(
		// 			_startOrigin.x + (_endOrigin.x - _startOrigin.x) * t,
		// 			_startOrigin.y + (_endOrigin.y - _startOrigin.y) * t,
		// 			1 / (1 / _startScale + (1 / _endScale - 1 / _startScale) * t))
		// 	}

		// 	_invalidate
		// }

		if(this._needsRedraw) {
            this._idleFrames = 0;
			this._isInvalid = false;
            this._draw();
            return;
		}
		// Render occasionally even when idle. Chrome must render at least 10fps to
		// avoid stutter when starting to render at 60fps again.
        this._idleFrames ++;
        if(this._idleFrames % 6 == 0 && this._idleFrames < 60 * 2) {
			this._draw();
		}
	}

    _draw(){
        this._canvas.start_frame();
        this._canvas.draw_grid();
        this._canvas.render();
        // _canvas.endFrame
    }
}

