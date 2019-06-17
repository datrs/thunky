use std::sync::{Arc, Mutex};

type Cb<T, E> = Box<Fn(&Result<T, E>) -> () + Send + Sync>;
type RunCb<T, E> = Box<Fn(&Thunky<T, E>) -> () + Send + Sync>;

pub struct Thunky<T, E> {
  run: RunCb<T, E>,
  state: Mutex<Option<Box<State<T, E> + Send + Sync>>>,
  stack: Mutex<Vec<Cb<T, E>>>,  
  cache: Mutex<Option<Result<T, E>>>
}

impl<T, E> Thunky<T, E> {
  /// Create a thunky instance with a function which take a reference of thunky as parameters,
  ///
  /// So we can call `thunky.cache()` in this function.    
  /// # Examples
  ///  
  /// ```
  /// // test: run_only_once
  /// extern crate thunky; 
  /// use thunky::*;
  /// use std::sync::Mutex;
  ///
  /// let v = Mutex::new(0);
  ///
  /// let run = move |thunk: &Thunky<u32, &str>| {
  ///   *v.lock().unwrap() += 1;
  ///   thunk.cache(Ok(*v.lock().unwrap()));
  /// };
  ///
  /// let thunk = Thunky::new(Box::new(run));
  /// 
  /// thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
  ///   assert_eq!(1, arg.unwrap());
  /// }));
  ///
  /// thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
  ///   assert_eq!(1, arg.unwrap());
  /// }));  
  ///
  /// thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
  ///   assert_eq!(1, arg.unwrap());
  /// }));
  /// ```  
  pub fn new (run: RunCb<T, E>) -> Arc<Thunky<T, E>> {
    Arc::new(Thunky {
      run,
      state: Mutex::new(Some(Box::new(Run {}))),      
      stack: Mutex::new(Vec::new()),      
      cache: Mutex::new(None)
    })
  }

  /// Set cache, if incoming is `Ok(T)`, cahce will be preserved, and ignore the following `thunk.cache()`.
  ///
  /// otherwise cache will be reset by next calling to `thunk.cache()`
  /// 
  /// # Examples
  ///
  /// ```
  /// // test: re_run_on_error
  ///  
  /// extern crate thunky;
  /// use std::sync::Mutex;
  /// use thunky::*;
  ///
  ///
  /// let v = Mutex::new(0);
  ///
  /// let run = move |thunk: &Thunky<u32, &str>| {
  ///   *v.lock().unwrap() += 1;
  ///
  ///   if *v.lock().unwrap() == 1 {
  ///     thunk.cache(Err("stop"))
  ///   } else if *v.lock().unwrap() == 2 {
  ///     thunk.cache(Err("stop"))
  ///   } else if *v.lock().unwrap() == 3 {
  ///     thunk.cache(Ok(*v.lock().unwrap()))
  ///   } else if *v.lock().unwrap() == 4 {
  ///     thunk.cache(Ok(*v.lock().unwrap()))
  ///   }
  /// };
  ///
  /// let thunk = Thunky::new(Box::new(run));
  /// 
  /// thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
  ///   assert_eq!("stop", arg.unwrap_err());
  /// }));
  /// 
  /// thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
  ///   assert_eq!("stop", arg.unwrap_err());
  /// }));  
  ///
  /// thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
  ///   assert_eq!(3, arg.unwrap());
  /// }));    
  ///
  /// thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
  ///   assert_eq!(3, arg.unwrap());
  /// }));  
  /// ```
  pub fn cache(&self, a: Result<T, E>) -> () {    
    while self.stack.lock().unwrap().len() > 0 {
      let cb = self.stack.lock().unwrap().pop().unwrap();
      cb(&a);
    }

    #[allow(unused_assignments)]
    let mut is_cached = false;

    match self.cache.lock().unwrap().as_ref() {
      Some(v) => {
        if v.is_ok() {
          is_cached = true
        } else {
          is_cached = false
        }
      },
      None => {
        is_cached = false
      }
    }

    if !is_cached {
      *self.cache.lock().unwrap() = Some(a);
    }
  }

  /// Call `run()` of the current state of thunky. There're three private inner states in thunky:
  ///
  /// ` Run {} `: initial state, and after set the cache to `Err(E)`, state turns back to `Run`.
  ///
  /// ` Wait {} `: after call `run()` of `Run`, before set the cache, state stays `Wait`.
  ///
  /// ` Finish {} `: after set the cache to `Ok(T)`, state turns to `Finish` forever.     
  ///
  ///
  /// # Examples
  ///  
  /// ```  
  /// // test: run only once async
  ///
  /// extern crate thunky;
  /// extern crate tokio;
  ///
  /// use std::sync::{ Arc, Mutex };
  /// use std::time::{Duration, Instant};
  /// use tokio::prelude::*;
  /// use tokio::timer::Delay;
  /// use thunky::*;  
  ///  
  /// let run = move |_thunk: &Thunky<u32, &str>| {};
  ///
  /// let thunk = Thunky::new(Box::new(run));
  ///
  /// let v = Mutex::new(0);
  /// let thunk_clone = Arc::clone(&thunk);
  /// let when = Instant::now() + Duration::from_millis(1000);    
  /// let task = Delay::new(when)
  ///   .and_then(move |_| {
  ///     *v.lock().unwrap() += 1;
  ///     thunk_clone.cache(Ok(*v.lock().unwrap()));
  ///     Ok(())
  ///   })
  ///   .map_err(|e| panic!("delay errored; err={:?}", e));  
  ///
  /// thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
  ///   assert_eq!(1, arg.unwrap());
  /// }));
  ///
  /// thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
  ///   assert_ne!(2, arg.unwrap());
  /// }));  
  ///
  /// thunk.run(Box::new(|arg: &Result<u32, &str>| -> () {
  ///   assert_eq!(1, arg.unwrap());
  /// }));
  ///
  /// tokio::run(task);  
  /// ```    
  pub fn run(&self, callback: Cb<T, E>) -> () {
    let state = self.state.lock().unwrap().take().unwrap();
    state.run(self, callback)
  }
}

trait State<T, E> {
  fn run(&self, thunky: &Thunky<T, E>, callback: Cb<T, E>) -> ();
}

struct Run {}

impl<T, E> State<T, E> for Run {
  fn run (&self, thunky: &Thunky<T, E>, callback: Cb<T, E>) -> () {
    thunky.stack.lock().unwrap().push(callback);     
    (thunky.run)(thunky);

    match thunky.cache.lock().unwrap().as_ref() {
      Some(cache) => {
        if cache.is_ok() {
          *thunky.state.lock().unwrap() = Some(Box::new(Finish {}));
        } else if cache.is_err() {       
          *thunky.state.lock().unwrap() = Some(Box::new(Run {}));          
        }
      },
      None => {
        *thunky.state.lock().unwrap() = Some(Box::new(Wait {}));
      }      
    }
  }
}

struct Wait {}

impl<T, E> State<T, E> for Wait {
  fn run (&self, thunky: &Thunky<T, E>, callback: Cb<T, E>) -> () {   
    thunky.stack.lock().unwrap().push(callback);
    *thunky.state.lock().unwrap() = Some(Box::new(Wait {}));
  }
}

struct Finish {}

impl<T, E> State<T, E> for Finish {
  fn run (&self, thunky: &Thunky<T, E>, callback: Cb<T, E>) -> () { 
    while thunky.stack.lock().unwrap().len() > 0 {
      let cb = thunky.stack.lock().unwrap().pop().unwrap();
      cb(thunky.cache.lock().unwrap().as_ref().unwrap());
    }
    callback(thunky.cache.lock().unwrap().as_ref().unwrap());
    *thunky.state.lock().unwrap() = Some(Box::new(Finish {}));
  }
}