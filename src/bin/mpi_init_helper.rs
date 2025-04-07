use std::process::Command;
use std::io::Result;

fn main() -> Result<()> {
    // When running MPI initialization, use the hostfile
    std::env::set_var("OMPI_MCA_orte_default_hostfile", "/etc/mpi-hostfile");
    
    // Optionally, you can test MPI connectivity here
    let output = Command::new("mpirun")
        .args(&["-np", "2", "--hostfile", "/etc/mpi-hostfile", "hostname"])
        .output()?;
    
    println!("MPI test output: {}", String::from_utf8_lossy(&output.stdout));
    
    Ok(())
}