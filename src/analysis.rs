#![allow(dead_code)]

mod utils {
    use std::{io::Write, fs::File};

    pub fn write_val_vec_to_file<T: std::fmt::Debug> (list: &Vec<T>, dir: &String) {
        let mut f = File::create(dir).unwrap();
        for fpr in list {
            let output = format!("{:?}\n", fpr);
            let _ = f.write_all(&output.as_bytes());
        }
    }
}

mod traceability {
    use std::{collections::HashMap, io::{BufReader, BufRead}, f64, fs};

    pub fn import_csv(file_dir: &String) -> HashMap<usize, (usize,f64)> {
        let file = fs::File::open(file_dir).unwrap();
        let reader = BufReader::new(file);
        let mut id_val_map = HashMap::<usize, (usize,f64)>::new();
        for line in reader.lines() {
            let str_line = line.unwrap();
            let items: Vec<&str> = str_line.split(",").collect();
            (items[0] != "id").then(|| {
                let id = items[0].to_string().parse::<usize>().unwrap();
                let inf = items[3].to_string().parse::<usize>().unwrap();
                let fuz = items[2].to_string().parse::<f64>().unwrap();
                id_val_map.insert(id, (inf,fuz));
            });
        }
        id_val_map
    }

    pub fn find_thd_fpr(hmap: &HashMap<usize,(usize,f64)>) -> Vec::<(f64, f64)> {
        let max_inf = find_max_inf_level(&hmap);
        let mut thd_fpr_list = Vec::<(f64, f64)>::new();
        let mut fuz_list: Vec<f64> = hmap.iter()
            .filter(|(_,(inf,_))| *inf == max_inf)
            .map(|(_, (_,fuz))| *fuz)
            .collect();

        fuz_list.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut i = 0;
        while *fuz_list.get(i).unwrap() != 100.00 {
            (i != 0).then(|| {
                while *fuz_list.get(i).expect(&i.to_string()) == *fuz_list.get(i-1).unwrap() {
                    i += 1;
                }
            });

            let fuz_min: f64 = *fuz_list.get(i).unwrap();
            let inf_list: Vec<usize> = hmap.iter()
                .filter(|(_,(_,fuz))| *fuz >= fuz_min)
                .map(|(_, (inf,_))| *inf)
                .collect();

            let fpr = inf_list.iter().filter(|inf| **inf == 0).count() as f64 / inf_list.len() as f64;

            i += 1;
            thd_fpr_list.push((fuz_min, fpr));
        }
        thd_fpr_list
    }

    pub fn calc_thd_fpr(hmap: &HashMap<usize,(usize,f64)>, thd_list: &Vec<f64>) -> Vec::<(usize, usize, f64)> {
        let mut fpr_list = Vec::<(usize, usize, f64)>::new();

        thd_list.iter().for_each(|thd| {
            let inf_list: Vec<usize> = hmap.iter()
                .filter(|(_,(_,fuz))| fuz >= thd)
                .map(|(_, (inf,_))| *inf)
                .collect();

            let item = match inf_list.len() == 0 {
                true => (0, 0, 0.0),
                false => {
                    let all_count = inf_list.len();
                    let fp_count = inf_list.iter().filter(|x| **x==0).count();
                    let fpr = (fp_count as f64) / (all_count as f64);
                    (fp_count, all_count, fpr)
                },
            };
            fpr_list.push(item);
        });
        fpr_list
    }

    pub fn calc_inf_dist(hmap: &HashMap<usize,(usize,f64)>) -> Vec::<(usize,usize)> {
        let max_inf = find_max_inf_level(hmap);
        let mut inf_list = Vec::<(usize,usize)>::new();

        for i in 0..max_inf {
            let count: usize = hmap.iter()
                .filter(|(_, (inf,_))| *inf == i + 1)
                .map(|(_, (inf,_))| *inf)
                .collect::<Vec<usize>>()
                .len();
            inf_list.push((i, count));
        }
        inf_list
    }

    pub fn calc_inf_fpr(hmap: &HashMap<usize,(usize,f64)>, thd_list: &Vec<f64>) -> Vec::<Vec<f64>> {
        let max_inf = find_max_inf_level(hmap);
        let real_inf_list: Vec<usize> = calc_inf_dist(hmap).iter().map(|(_,x)| *x).collect();
        let mut thd_fill_rate_list = Vec::<Vec<f64>>::new();
        
        thd_list.iter().for_each(|thd| {
            let mut fill_rate_list = Vec::<f64>::new();
            let mut thd_inf_list = Vec::<usize>::new();
            for i in 0..max_inf {
                let count: usize = hmap.iter()
                    .filter(|(_,(_,fuz))| *fuz >= *thd)
                    .filter(|(_, (inf,_))| *inf == i + 1)
                    .map(|(_, (inf,_))| *inf)
                    .collect::<Vec<usize>>()
                    .len();
                thd_inf_list.push(count);
            }

            for i in 0..max_inf {
                let fill_rate = (*thd_inf_list.get(i).unwrap() as f64) / (*real_inf_list.get(i).unwrap() as f64);
                fill_rate_list.push(fill_rate);
            }

            thd_fill_rate_list.push(fill_rate_list);
        });
        thd_fill_rate_list
    }

    pub fn calc_fuz_fpr(hmap: &HashMap<usize,(usize,f64)>, thd_list: &Vec<f64>) -> Vec::<(usize, usize, f64)> {
        let mut fuz_fpr_list = Vec::<(usize, usize, f64)>::new();

        for i in 0..(thd_list.len()-1) {
            let inf_list: Vec<usize> = hmap.iter()
                .filter(|(_,(_,fuz))| *fuz >= *thd_list.get(i).unwrap())
                .filter(|(_,(_,fuz))| *fuz < *thd_list.get(i+1).unwrap())
                .map(|(_,(inf,_))| *inf)
                .collect();
            let all_count = inf_list.len();
            let fp_count = inf_list.iter().filter(|x| **x==0).count();
            let fpr = (fp_count as f64) / (all_count as f64);
            fuz_fpr_list.push((fp_count, all_count, fpr));
        }

        fuz_fpr_list
    }

    pub fn gen_thd_list() -> Vec::<f64>{
        let mut thd_list = Vec::<f64>::new();
        let (mut lower_bound, upper_bound, step) = (99.99, 100.0, 0.003);

        while lower_bound < upper_bound {
            let input = f64::trunc(lower_bound * 1000.0) / 1000.0;
            thd_list.push(input);
            lower_bound += step;
        }
        thd_list.extend(vec![99.9995, 99.99995, 99.999995, 99.9999995, 99.99999995, 99.999999995]);
        thd_list.push(upper_bound);
        thd_list
    }

    fn find_max_inf_level(hmap: &HashMap<usize,(usize,f64)>) -> usize {
        let inf_list: Vec<usize> = hmap.iter()
                .map(|(_, (inf,_))| *inf)
                .collect();
        inf_list.into_iter().max().unwrap()
    }

}

mod tests {
    use crate::analysis::traceability::calc_inf_fpr;

    use super::{traceability::{find_thd_fpr, import_csv, calc_thd_fpr, calc_inf_dist, gen_thd_list, calc_fuz_fpr}, utils::write_val_vec_to_file};

    extern crate test;

    #[test]
    fn test_inf_dist(){
        let list = import_csv(&"../Traceability-Evaluation/inputs/fuz_val_and_inf.csv".to_string());
        write_val_vec_to_file(&calc_inf_dist(&list), &"./output/inf_dist/k_shell.txt".to_string());
    }

    #[test]
    fn test_thd_fpr(){
        let thd_list = gen_thd_list();
        let val_list = import_csv(&"../Traceability-Evaluation/inputs/fuz_val_and_inf.csv".to_string());
        write_val_vec_to_file(&calc_thd_fpr(&val_list, &thd_list), &"./output/thd_fpr_fix_step/thd_fpr.txt".to_string());
    }

    #[test]
    fn test_inf_fpr() {
        let thd_list = gen_thd_list();
        let val_list = import_csv(&"../Traceability-Evaluation/inputs/fuz_val_and_inf.csv".to_string());
        write_val_vec_to_file(&calc_inf_fpr(&val_list, &thd_list), &"./output/inf_fpr_dist/inf_fpr.txt".to_string());
    }

    #[test]
    fn test_fuz_fpr() {
        let thd_list = vec![0.0, 80.0, 90.0, 95.0, 99.0, 99.5, 99.9, 99.99];
        let val_list = import_csv(&"../Traceability-Evaluation/inputs/fuz_val_and_inf.csv".to_string());
        write_val_vec_to_file(&calc_fuz_fpr(&val_list, &thd_list), &"./output/fuz_fpr/fuz_fpr.txt".to_string());
    }
}