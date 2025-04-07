use mpi::traits::*;
use mpi::environment::Threading;

fn main() {
    // Attempt to initialize MPI and print diagnostic info
    println!("Attempting to initialize MPI with thread support...");
    
    // Try with thread support first
    match mpi::initialize_with_threading(Threading::Multiple) {
        Some((universe, provided_threading)) => run_mpi_test(universe, Some(provided_threading)),
        None => {
            println!("MPI initialization with thread support failed");
            println!("Trying without thread support...");
            
            // Fall back to no thread support
            match mpi::initialize() {
                Some(universe) => run_mpi_test(universe, None),
                None => {
                    println!("MPI initialization failed!");
                    println!("Possible reasons:");
                    println!("1. OpenMPI might not be properly installed or configured");
                    println!("2. Environment variables might not be set (OMPI_ALLOW_RUN_AS_ROOT=1)");
                    println!("3. SSH might not be running (required for MPI communication)");
                    println!("4. Try running with: mpirun -np 2 --allow-run-as-root cargo run --bin mpi_test");
                    
                    // Try to detect the MPI environment
                    detect_mpi_environment();
                }
            }
        }
    }
}

fn run_mpi_test(universe: mpi::environment::Universe, threading: Option<Threading>) {
    let world = universe.world();
    let rank = world.rank();
    let size = world.size();
    
    if let Some(threading) = threading {
        println!("MPI initialized successfully with thread support level: {:?}", threading);
    } else {
        println!("MPI initialized successfully without thread support");
    }
    
    println!("Process {} of {} on this node", rank, size);
    
    // Try basic communication if more than one process
    if size > 1 {
        if rank == 0 {
            // Master sends a message to worker
            let msg = vec![42; 10];
            println!("Master sending data to process 1");
            world.process_at_rank(1).send(&msg[..]);
            println!("Send complete");
        } else if rank == 1 {
            // Worker receives message
            println!("Worker waiting to receive data");
            let (msg, _) = world.process_at_rank(0).receive_vec::<i32>();
            println!("Worker received data: {:?}", &msg[0..3]);
        }
    }
    
    // Synchronize all processes
    println!("Process {} entering barrier", rank);
    world.barrier();
    
    if rank == 0 {
        println!("All processes completed successfully!");
    }
}

fn detect_mpi_environment() {
    println!("\nDetecting MPI environment...");
    
    // Check if mpirun is available
    if let Ok(output) = std::process::Command::new("which").arg("mpirun").output() {
        if !output.stdout.is_empty() {
            println!("mpirun found at: {}", String::from_utf8_lossy(&output.stdout).trim());
        } else {
            println!("mpirun not found in PATH");
        }
    } else {
        println!("Failed to check for mpirun");
    }
    
    // Check OpenMPI version
    if let Ok(output) = std::process::Command::new("mpirun").arg("--version").output() {
        println!("MPI version: {}", String::from_utf8_lossy(&output.stdout).trim());
    } else {
        println!("Failed to check MPI version");
    }
    
    // Check if SSH is running
    if let Ok(output) = std::process::Command::new("ps").args(&["-ef"]).output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains("sshd") {
            println!("SSH daemon is running");
        } else {
            println!("SSH daemon does not appear to be running");
        }
    } else {
        println!("Failed to check SSH daemon status");
    }
    
    // Check environment variables
    if let Some(value) = std::env::var_os("OMPI_ALLOW_RUN_AS_ROOT") {
        println!("OMPI_ALLOW_RUN_AS_ROOT={}", value.to_string_lossy());
    } else {
        println!("OMPI_ALLOW_RUN_AS_ROOT is not set");
    }
    
    if let Some(value) = std::env::var_os("OMPI_ALLOW_RUN_AS_ROOT_CONFIRM") {
        println!("OMPI_ALLOW_RUN_AS_ROOT_CONFIRM={}", value.to_string_lossy());
    } else {
        println!("OMPI_ALLOW_RUN_AS_ROOT_CONFIRM is not set");
    }
}